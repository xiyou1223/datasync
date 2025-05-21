# 一个便于内部使用的数据库同步工具

目前实现了 mysql 的整库迁移功能，使用 mysqldump 和 mysql 命令进行实现。

## 使用方法

1. 参考 job 文件夹下的 job.yml.example 文件，编写自己的任务
2. 运行 cargo run ./job/job.yml
