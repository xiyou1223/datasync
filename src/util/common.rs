use std::{error::Error, fs};

use serde::Deserialize;

// 载入任务配置
// 配置文件为toml格式
// 任务配置文件的路径为：/job/{name}.toml
pub fn load_job_config<T>(toml_path: &str) -> Result<T, Box<dyn Error>>
where
    for<'de> T: Deserialize<'de>,
{
    println!("Loading job config from: {}", toml_path);
    let job_str = fs::read_to_string(toml_path)?;
    let job: T = toml::from_str(&job_str)?;
    Ok(job)
}
