use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    process::Command,
    sync::Arc,
};

use chrono::Local;
use encoding::label::encoding_from_whatwg_label;
use sqlx::Row;
use tokio::sync::Semaphore;

use crate::model::job::{Source, Target};

#[derive(Clone, Debug)]
pub struct MysqlHelp {
    pub source_pool: Arc<sqlx::Pool<sqlx::MySql>>,
    pub target_pool: Arc<sqlx::Pool<sqlx::MySql>>,
}

impl MysqlHelp {
    pub fn new(
        source_pool: Arc<sqlx::Pool<sqlx::MySql>>,
        target_pool: Arc<sqlx::Pool<sqlx::MySql>>,
    ) -> Self {
        MysqlHelp {
            source_pool,
            target_pool,
        }
    }

    pub async fn get_mysql_version(&self) -> Result<Vec<String>, sqlx::Error> {
        let source_row: (String,) = sqlx::query_as("SELECT VERSION()")
            .fetch_one(&*self.source_pool)
            .await?;
        let target_row: (String,) = sqlx::query_as("SELECT VERSION()")
            .fetch_one(&*self.target_pool)
            .await?;
        let source_version = source_row.0;
        let target_version = target_row.0;
        Ok(vec![
            format!("source version: {}", source_version),
            format!("target version: {}", target_version),
        ])
    }

    pub async fn get_all_databases(&self) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query("SHOW DATABASES")
            .fetch_all(&*self.source_pool)
            .await?;
        let mut databases = Vec::new();
        for row in rows {
            let db_name: String = row.get("Database");
            databases.push(db_name);
        }
        Ok(databases)
    }

    // 同步所有数据库
    pub async fn sync_all_db(&self, source: &Source, target: &Target) -> Result<(), sqlx::Error> {
        self.backup_all_db(source, target).await
    }

    // 备份所有数据库
    pub async fn backup_all_db(&self, source: &Source, target: &Target) -> Result<(), sqlx::Error> {
        let databases = self.get_all_databases().await?;
        // 设置最大并发数为 5
        let semaphore = Arc::new(Semaphore::new(5));
        let mut tasks = Vec::new();

        for db_name in databases {
            let source_cloned = source.clone();
            let target_cloned = target.clone();
            // let help = self.clone();
            let help_arc = Arc::new(self.clone());

            let permit = semaphore.clone().acquire_owned().await.unwrap();
            let task = tokio::spawn(async move {
                let backup_file_path = help_arc
                    .mysqldump_database_backup(&source_cloned, &db_name)
                    .await;

                // 备份成功则还原
                if backup_file_path.is_ok() {
                    let backup_file_path = backup_file_path.unwrap();
                    help_arc
                        .mysqldump_database_restore(&backup_file_path, &target_cloned, &db_name)
                        .await;
                }
                // 释放信号量
                drop(permit);
            });
            tasks.push(task);
        }
        // 等待所有任务完成
        for h in tasks {
            if let Err(e) = h.await {
                eprintln!("Task failed: {}", e);
            }
        }
        Ok(())
    }

    // 还原数据库
    pub async fn mysqldump_database_restore(
        &self,
        backup_file_path: &str,
        target: &Target,
        db_name: &str,
    ) {
        // 判断数据库是否存在，不存在则创建
        let db_name = db_name.trim();
        let db_exists = sqlx::query(
            "SELECT SCHEMA_NAME FROM information_schema.SCHEMATA WHERE SCHEMA_NAME = ?",
        )
        .bind(db_name)
        .fetch_optional(&*self.target_pool)
        .await
        .expect("Failed to check database existence");

        if db_exists.is_none() {
            sqlx::query(format!("CREATE DATABASE IF NOT EXISTS `{}`", db_name).as_str())
                .execute(&*self.target_pool)
                .await
                .expect("Failed to create database");
            println!("Database {} created", db_name);
        } else {
            println!("Database {} already exists", db_name);
        }

        // 执行mysql命令，还原数据库
        let output = execute_mysql_restore(
            backup_file_path,
            &target.host,
            &target.port,
            &target.user,
            &target.password,
            db_name,
        )
        .expect("Failed to execute mysql restore command");

        if !output.status.success() {
            let decoded_stderr = decode_stderr(&output.stderr);
            eprintln!("mysql restored failed: {}", decoded_stderr,);
        } else {
            println!("[ok] Database restored from {}", backup_file_path);
        }
    }

    // 备份数据库
    pub async fn mysqldump_database_backup(
        &self,
        source: &Source,
        db_name: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // mysqldump
        // 构造备份文件路径
        let time_str = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let output_file_path = format!("sql/backup_{}_{}.sql", db_name, time_str);

        if let Some(parent) = Path::new(&output_file_path).parent() {
            fs::create_dir_all(parent).expect("Failed to create directory");
        }

        let mut output_file = File::create(&output_file_path).expect("Failed to create file");

        // 执行mysqldump命令
        let output = Command::new("mysqldump")
            .arg(format!("--user={}", source.user))
            .arg(format!("--password={}", source.password))
            .arg(format!("--host={}", source.host))
            .arg(format!("--port={}", source.port))
            .arg("--compression-algorithms=zlib") // 压缩输出
            .arg("--single-transaction") // 一致性事务快照
            .arg("--set-gtid-purged=OFF")
            .arg("--triggers") // 备份触发器
            .arg("--routines") // 备份存储过程和函数
            .arg("--events") // 备份事件
            .arg(db_name)
            .output()?;

        // 写入到文件
        output_file.write_all(&output.stdout)?;

        if !output.status.success() {
            eprintln!(
                "mysqldump failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            return Err("mysqldump failed".into());
        } else {
            println!("[ok] Database {} dumped to {}", db_name, output_file_path);
        }

        return Ok(output_file_path);
    }
}

#[cfg(target_family = "unix")]
fn execute_mysql_restore<P: AsRef<Path>>(
    backup_file: P,
    host: &str,
    port: &str,
    user: &str,
    password: &str,
    db_name: &str,
) -> std::io::Result<std::process::Output> {
    Command::new("mysql")
        .arg("--default-character-set=utf8")
        .arg(format!("-h{}", host))
        .arg(format!("-P{}", port))
        .arg(format!("-u{}", user))
        .arg(format!("-p{}", password))
        .arg(db_name)
        .arg("<")
        .arg(backup_file.as_ref())
        .output()
}

#[cfg(target_family = "windows")]
fn execute_mysql_restore<P: AsRef<Path>>(
    backup_file: P,
    host: &str,
    port: &str,
    user: &str,
    password: &str,
    db_name: &str,
) -> std::io::Result<std::process::Output> {
    let cmd_str = format!(
        "mysql --default-character-set=utf8 -h{} -P{} -u{} -p{} {} < {}",
        host,
        port,
        user,
        password,
        db_name,
        backup_file.as_ref().to_str().unwrap()
    );
    // println!("Executing command: {}", cmd_str);

    Command::new("cmd")
        .arg("/c")
        .arg(format!("{}", cmd_str))
        .output()
}

// 解码错误的输出
// 解决中文乱码问题（windows下）
pub fn decode_stderr(data: &[u8]) -> String {
    let encoding = encoding_from_whatwg_label("gbk").unwrap_or_else(|| encoding::all::UTF_8);
    encoding
        .decode(data, encoding::DecoderTrap::Replace)
        .unwrap_or_else(|_| String::from_utf8_lossy(data).to_string())
}
