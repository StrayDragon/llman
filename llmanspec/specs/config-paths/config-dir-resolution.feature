# language: zh-CN
# 对应 spec: config-paths — CLI MUST 按优先级解析配置目录：CLI --config-dir >
# LLMAN_CONFIG_DIR 环境变量 > 默认 ~/.config/llman；解析值 MUST 赋给 LLMAN_CONFIG_DIR；
# 解析 MUST NOT 创建目录。
功能: 配置目录按优先级解析
  场景: CLI 覆盖优先且传播给子命令
    假如 用户运行带 --config-dir 的命令
    当 解析配置目录
    那么 解析结果为 CLI 值
    而且 LLMAN_CONFIG_DIR 被设置给子命令

  场景: 环境变量覆盖次之
    假如 设置了 LLMAN_CONFIG_DIR 且未提供 CLI 覆盖
    当 解析配置目录
    那么 解析结果为环境变量值

  场景: 无覆盖时回退默认路径且不创建目录
    假如 未提供 CLI 或环境变量覆盖
    当 解析配置目录
    那么 解析结果为 {home}/.config/llman
    而且 而且解析过程不创建目录

  场景: macOS 旧版目录不被默认解析采纳
    假如 用户在 macOS 上无 CLI/env 覆盖运行命令
    而且 {home}/Library/Application Support/llman 或 com.StrayDragon.llman 含可识别配置根
    当 默认解析配置目录
    那么 解析结果为 {home}/.config/llman
