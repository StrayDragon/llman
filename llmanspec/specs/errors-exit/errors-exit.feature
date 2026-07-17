# language: zh-CN
# Partitioned harness for errors-exit — unique scenarios only.
# Bound CLI scenarios live in error-rendering.feature / subcommand-error-handling.feature.
功能: errors-exit

  @req:r2
  场景: json-错误输出
    假如 llman 二进制已构建
    当 运行 llman sdd show nonexistent --type spec --json
    那么 退出码为 1
    那么 stderr 包含 Error
