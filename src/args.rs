// 处理命令行参数
pub mod args_handle {
    use std::env::Args;

    #[derive(Debug)]
    pub struct ArgsConfig {
        pub job_config_path: String,
    }

    pub trait PrintMe: std::fmt::Debug {
        fn dump(&self);
    }

    impl PrintMe for ArgsConfig {
        fn dump(&self) {
            println!("args config: {:?}", self);
        }
    }

    impl ArgsConfig {
        pub fn build(mut args: Args) -> Result<Self, &'static str> {
            args.next();
            let job_config_path = match args.next() {
                Some(path) => path,
                None => return Err("No job config path provided"),
            };
            let job_config = ArgsConfig { job_config_path };
            Ok(job_config)
        }
    }
}

#[cfg(test)]
mod test_job_config {
    use crate::model::job::JobModel;
    use crate::util::common as util_common;

    #[test]
    fn test_read_job_config() {
        // 读取任务配置文件
        let current_dir = std::env::current_dir().expect("Failed to get current directory");
        let job_config_path = current_dir.join("job").join("canteen_all_db_sync.toml");
        let job_config_path_str = job_config_path
            .to_str()
            .expect("Failed to convert path to string");
        let job: JobModel =
            util_common::load_job_config(job_config_path_str).expect("Failed to load job config");
        println!("{:?}", job);
    }
}
