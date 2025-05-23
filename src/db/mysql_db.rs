use std::collections::HashMap;
use std::sync::Arc;
use std::sync::LazyLock;

use sqlx::MySql;
use sqlx::Pool;
use sqlx::mysql::MySqlPoolOptions;
use tokio::sync::Mutex;

// 使用一个map来存储数据库连接池
// 使用ones_cell.Lazy来实现单例模式(hashmap延时初始化)
pub static MYSQL_DB_POOLS: LazyLock<Mutex<HashMap<String, Arc<Pool<MySql>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// 初始化数据库连接池
pub async fn init_mysql_db_pool(dns: &str, pool_name: &str) -> Result<(), String> {
    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect_lazy(dns)
        .expect("Failed to create pool");

    // 检查db_name是否已经存在
    if MYSQL_DB_POOLS.lock().await.contains_key(pool_name) {
        // return Err(format!("Database pool for {} already exists", pool_name));
        println!("Database pool for {} already exists", pool_name);
        return Ok(());
    }

    let pool_arc = Arc::new(pool);
    MYSQL_DB_POOLS
        .lock()
        .await
        .insert(pool_name.to_string(), pool_arc);

    return Ok(());
}
