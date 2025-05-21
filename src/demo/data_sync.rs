use std::{
    fs::{self, File},
    io::Write,
    path::Path,
    process::Command,
    sync::Arc,
};

use chrono::Local;
use sqlx::Row;
use sqlx::mysql::MySqlPoolOptions;
use tokio::sync::OnceCell;

static DB_POOL: OnceCell<Arc<sqlx::MySqlPool>> = OnceCell::const_new(); // once_cell单例模式

pub fn init_db_pool() {
    let database_url = "mysql://root:root@localhost:3306";
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect_lazy(database_url)
        .expect("Failed to create pool");
    let arc_pool = Arc::new(pool);
    DB_POOL
        .set(arc_pool.clone())
        .expect("DB_POOL already initialized");
}

pub fn get_db_pool() -> Arc<sqlx::MySqlPool> {
    DB_POOL.get().expect("DB_POOL not initialized").clone()
}

pub async fn show_database() {
    let dbs = get_databases().await;
    println!("Databases: {:?}", dbs);
}

pub async fn get_databases() -> Vec<String> {
    let pool = get_db_pool();
    let result = sqlx::query("SHOW DATABASES").fetch_all(&*pool).await;
    let mut dbs = Vec::new();
    match result {
        Ok(rows) => {
            for row in rows {
                let db_name: String = row.get("Database");
                // println!("Database: {}", db_name);
                dbs.push(db_name);
            }
        }
        Err(e) => eprintln!("Error fetching databases: {}", e),
    }
    dbs
}

// 还原数据库
pub async fn mysqldump_database_restore(backup_file_path: &str) {
    println!("run mysql to restore database from: {}", backup_file_path);
}

// 备份数据库
pub async fn mysqldump_database_backup(db_name: &str) -> String {
    // println!("run mysqldump to database: {}", db_name);
    // mysqldump
    // 构造备份文件路径
    let time_str = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let output_file_path = format!("sql/backup_{}_{}.sql", db_name, time_str);
    // let gz_file_path = format!("sql/backup_{}_{}.sql.gz", db_name, time_str);

    if let Some(parent) = Path::new(&output_file_path).parent() {
        fs::create_dir_all(parent).expect("Failed to create directory");
    }

    let mut output_file = File::create(&output_file_path).expect("Failed to create file");

    // 创建gz文件
    // let gz_file = File::create(&gz_file_path).expect("Failed to create gz file");
    // let mut gz = GzEncoder::new(gz_file, flate2::Compression::default());

    // 执行mysqldump命令
    let output = Command::new("mysqldump")
        .arg("--user=root")
        .arg("--password=root")
        .arg("--host=localhost")
        .arg("--port=3306")
        .arg("--databases")
        .arg(db_name)
        .arg("--compress") // 压缩输出
        .arg("--single-transaction") // 一致性事务快照
        .arg("--routines") // 导出存储过程
        .arg("--events") // 导出事件
        .output()
        .expect("Failed to execute command");

    // 写入到文件
    output_file
        .write_all(&output.stdout)
        .expect("Failed to write to file");

    // 写入到gz文件
    // gz.write_all(&output.stdout)
    //     .expect("Failed to write to gz file");
    // gz.finish().expect("Failed to finish gz file");

    if !output.status.success() {
        eprintln!(
            "mysqldump failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    } else {
        println!("Database {} dumped to {}", db_name, output_file_path);
    }

    return output_file_path;
}
