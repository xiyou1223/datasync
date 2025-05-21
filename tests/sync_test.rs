use std::sync::Arc;

use datasync::demo::data_sync::{
    get_databases, init_db_pool, mysqldump_database_backup, mysqldump_database_restore,
    show_database,
};
use tokio::sync::Semaphore;

#[test]
fn test_hello() {
    println!("Hello, world!");
}

#[tokio::test]
async fn test_mysqldump() {
    init_db_pool();
    show_database().await;

    // test dump database
    // let db_name = "ts0002";
    // mysqldump_database(db_name).await;

    // dump all databases
    let semaphore = Arc::new(Semaphore::new(5)); // 限制并发数为5
    let mut tasks = Vec::new();
    for db in get_databases().await {
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let task = tokio::spawn(async move {
            let file_path = mysqldump_database_backup(&db).await;
            mysqldump_database_restore(&file_path).await; // 还原数据库
            // 释放信号量
            drop(permit);
        });
        tasks.push(task);
    }
    for task in tasks {
        if let Err(e) = task.await {
            eprintln!("Task failed: {}", e);
        }
    }
}
