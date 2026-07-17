# language: zh-CN
# 对应 spec: errors-exit r2 — show --json 对不存在 spec 的错误输出。
# Bound CLI scenarios live in error-rendering.feature / subcommand-error-handling.feature.
功能: show --json 错误输出
  @req:r53
  @executable
  场景: json-错误输出
    假如 llman 二进制已构建
    当 运行 llman sdd show nonexistent --type spec --json
    那么 退出码为 1
    那么 stderr 包含 Error
