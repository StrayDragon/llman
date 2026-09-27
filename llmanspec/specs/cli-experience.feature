# language: zh-CN
# capability: cli-experience
# purpose: 规范 llman CLI 的消息、本地化覆盖与 stdout/stderr 约定。
# scope: llmanspec/specs/cli-experience.feature

功能: cli-experience

  @req:r14
  规则: shell 补全生成与 install 安全写入
    PowerShell 写入目标 MUST 位于 home 下。

    场景: completion-help-surface
      当 运行 llman self completion --help
      那么 退出码为零
      那么 stdout 包含 powershell
  @req:r43
  规则: 本地化消息与 stdout/stderr 约定
    对应 spec: cli-experience — 运行时提示/状态/错误 MUST 优先用 t! 本地化键；locale 固定英文； 正常输出与交互提示到 stdout，错误到 stderr；单行消息使用一致前缀。

    场景: errors-go-to-stderr
      假如 llman 二进制已构建
      当 运行 llman no-such-cmd
      那么 退出码非零
      那么 stdout 不含 unrecognized
      那么 stderr 包含 unrecognized
