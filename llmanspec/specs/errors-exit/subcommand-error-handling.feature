# language: zh-CN
# 对应 spec: errors-exit r2 — 命令处理器 MUST 在致命错误时返回 Err；
# 交互流程 MAY 自行打印错误并直接退出；可恢复问题 MAY 记录到 stderr 但不使命令失败。
功能: 子命令错误处理
  场景: 非交互终端下 sdd show 无参数时以退出码 1 退出
    假如 llman 二进制已构建
    当 在非交互终端运行 llman sdd show
    那么 退出码为 1
    那么 stderr 包含 Nothing to show

  场景: 查看不存在的 spec 时正常报错而非 panic
    假如 llman 二进制已构建
    当 运行 llman sdd show nonexistent-spec --type spec
    那么 退出码非零
    那么 stderr 包含 Error
