# language: zh-CN
# 对应 spec: config-paths — 解析器 MUST 对空或空白 CLI/env 路径报错且不创建目录；
# read_with_max_size MUST 在文件超限时拒绝读取（默认 10 MiB）；
# 原子写入 MUST 在目标是 symlink 时先删除 symlink 再写新文件（不跟随 symlink）。
功能: 非法路径报错与安全 IO 边界
  场景: 空 CLI 路径报错
    假如 --config-dir 无法解析为合法路径
    当 运行命令
    那么 命令返回错误并由 CLI 入口呈现

  场景: 空白环境变量报错
    假如 LLMAN_CONFIG_DIR 设置为空或空白值
    当 运行命令
    那么 命令返回错误并由 CLI 入口呈现

  场景: 正常大小文件正常读取
    假如 配置文件大小 < 10 MiB
    当 read_with_max_size 读取
    那么 正常返回文件内容

  场景: 超大文件被拒绝
    假如 文件大小 > 10 MiB
    当 read_with_max_size 读取
    那么 返回错误并提示文件超过大小上限

  场景: 配置读取经限速入口而非直接 read_to_string
    假如 各 YAML/TOML 解析调用点读取配置文件
    当 读取发生
    那么 经 read_with_max_size 或其他限速入口

  场景: 原子写入不跟随 symlink
    假如 目标路径是指向其它位置的 symlink
    当 atomic_write_with_mode 执行
    那么 symlink 被替换为新文件
    而且 而且内容不写入链接目标
