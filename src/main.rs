use std::{env, sync::Arc};

use datasync::{
    args::args_handle::{ArgsConfig, PrintMe},
    db::mysql_db::{MYSQL_DB_POOLS, init_mysql_db_pool},
    handle::help::MysqlHelp,
    model::job::JobModel,
    util::common as util_common,
};

#[tokio::main]
async fn main() {
    // 处理命令行参数
    let args_result = ArgsConfig::build(env::args());
    let args_config = match args_result {
        Ok(cfg) => cfg,
        Err(e) => {
            println!("Error: {}", e);
            return;
        }
    };
    args_config.dump();

    // 读取任务配置文件
    let job = util_common::load_job_config::<JobModel>(&args_config.job_config_path)
        .expect("Failed to load job config");
    println!("任务配置内容：{:?}", job);

    // 匹配任务数据库类型
    match job.job.database_type.as_str() {
        "mysql" => {
            println!("--- mysql任务 ---");
            mysql_job_handle(job).await;
            return;
        }
        _ => {
            println!("暂不支持的数据库类型");
        }
    }
}

// 处理mysql任务
async fn mysql_job_handle(job: JobModel) {
    let job_name = job.job.name;
    match job.job.job_type.as_str() {
        "all_database_sync" => {
            println!("--- 全库同步任务 ---");
            println!("任务名称：{}", job_name);
            let source_dns = format!(
                "mysql://{}:{}@{}:{}/",
                job.source.user, job.source.password, job.source.host, job.source.port
            );
            let source_pool_name = if job.source.db_name.is_some() {
                format!(
                    "source_{}_{}",
                    job_name,
                    job.source.db_name.as_ref().unwrap()
                )
            } else {
                format!("source_{}_{}", job_name, "all")
            };
            let source_init_pool_result = init_mysql_db_pool(&source_dns, &source_pool_name);

            let target_dns = format!(
                "mysql://{}:{}@{}:{}/",
                job.target.user, job.target.password, job.target.host, job.target.port
            );
            let target_pool_name = if job.target.db_name.is_some() {
                format!(
                    "target_{}_{}",
                    job_name,
                    job.target.db_name.as_ref().unwrap()
                )
            } else {
                format!("target_{}_{}", job_name, "all")
            };
            let target_init_pool_result = init_mysql_db_pool(&target_dns, &target_pool_name);

            if source_init_pool_result.is_ok() && target_init_pool_result.is_ok() {
                println!(
                    "数据库连接池创建成功: source:{}, target:{}",
                    source_pool_name, target_pool_name
                );
                // 连接池创建成功，执行后续操作
                let pool_map = MYSQL_DB_POOLS.lock().unwrap();
                let source_pool_result = pool_map.get(&source_pool_name);
                if source_pool_result.is_none() {
                    println!("源数据库连接池获取失败: {}", source_pool_name);
                    return;
                }
                let source_pool = source_pool_result.unwrap();
                let source_pool_arc = Arc::new(source_pool.clone());

                let target_pool_result = pool_map.get(&target_pool_name);
                if target_pool_result.is_none() {
                    println!("目标数据库连接池获取失败: {}", source_pool_name);
                    return;
                }
                let target_pool = target_pool_result.unwrap();
                let target_pool_arc = Arc::new(target_pool.clone());

                // println!("===> pool: {:?}", pool);
                // 查询数据库版本信息
                let help = MysqlHelp::new(source_pool_arc, target_pool_arc);
                let versions = help.get_mysql_version().await;
                match versions {
                    Ok(ver) => println!("数据库版本信息: {:?}", ver),
                    Err(e) => {
                        println!("获取源数据库版本失败: {}", e);
                        return;
                    }
                }

                // 同步所有数据库
                println!("--- 开始同步所有数据库...");
                let backup_result = help.sync_all_db(&job.source, &job.target).await;
                match backup_result {
                    Ok(_) => println!("同步所有数据库成功"),
                    Err(e) => {
                        println!("同步所有数据库成功: {}", e);
                        return;
                    }
                }
            } else {
                println!(
                    "源数据库创建连接池失败: {}",
                    source_init_pool_result.err().unwrap()
                );
                return;
            }
        }
        _ => {
            println!("暂不支持的任务类型");
        }
    }
}
