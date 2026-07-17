# language: zh-CN
# 对应 spec: errors-exit r1 — 当子命令返回错误时，CLI MUST 向 stderr 输出
# 单条用户可见错误信息并以退出码 1 退出。
功能: CLI 入口错误渲染
  @req:r22
  @executable
  场景: 子命令返回错误时打印单行错误并以退出码 1 退出
    假如 llman 二进制已构建
    当 在非交互终端运行 llman sdd show
    那么 退出码为 1
    那么 stderr 包含 Error
