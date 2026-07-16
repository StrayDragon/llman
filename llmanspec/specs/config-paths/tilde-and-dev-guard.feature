# language: zh-CN
# 对应 spec: config-paths — 当 --config-dir 或 LLMAN_CONFIG_DIR 以 ~ 开头时 MUST 展开为
# 用户主目录；在 llman 开发仓库内运行且无覆盖时 MUST 报错要求显式指定。
功能: tilde 展开与开发仓库守卫
  场景: CLI 带引号的 tilde 路径正确展开
    假如 用户运行 llman --config-dir "~/.config/llman" {subcommand}
    当 解析配置目录
    那么 解析结果为 {home}/.config/llman
    而且 而且不是 ./{}/.config/llman

  场景: 环境变量 tilde 路径正确展开
    假如 LLMAN_CONFIG_DIR 设置为 "~/.config/llman" 且无 CLI 覆盖
    当 解析配置目录
    那么 解析结果为 {home}/.config/llman

  场景: 开发仓库内无覆盖时报错
    假如 当前目录含包名为 llman 的 Cargo.toml
    而且 而且未提供任何覆盖
    当 运行命令
    那么 命令以 config-dir-required 错误失败
