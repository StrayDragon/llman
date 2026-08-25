# language: zh-CN
# capability: cli-experience
# purpose: 规范 llman CLI 的消息、本地化覆盖与 stdout/stderr 约定。
# scope: llmanspec/specs/cli-experience

功能: cli-experience

  @req:r14 @human
  场景: shell 补全生成与 install 安全写入
    - PowerShell 写入目标 MUST 位于 home 下。

  @req:r43 @human
  场景: 本地化消息与 stdout/stderr 约定
    - 对应 spec: cli-experience — 运行时提示/状态/错误 MUST 优先用 t! 本地化键；locale 固定英文； 正常输出与交互提示到 stdout，错误到 stderr；单行消息使用一致前缀。
