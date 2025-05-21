// 整库迁移

use serde::Deserialize;

// 声明任务配置结构
#[derive(Debug, Deserialize, Clone)]
pub struct JobModel {
    pub job: Job,
    pub source: Source,
    pub handler: Option<Handler>,
    pub target: Target,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Job {
    pub name: String,
    #[serde(rename = "type")]
    pub job_type: String,
    pub database_type: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Source {
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub db_name: Option<String>, // 允许不配置
    pub table_name: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Handler {
    // 如果 handler 是空对象，也可以用占位字段替代
    #[serde(skip_deserializing)]
    pub placeholder: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Target {
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub db_name: Option<String>,
    pub table_name: Option<String>,
}

pub async fn all_database_sync() {}
