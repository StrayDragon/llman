# language: zh-CN
# 对应 spec: claude-code-runner — llman x cc/claude-code run 交互模式收集参数 MUST 支持引号解析
# （未闭合引号报错且不执行）；主命令 MUST 接受通过 -- 分隔的 trailing args 原样透传给 claude。
功能: 交互参数引号解析与 -- 参数透传
  @req:r12
  场景: 引号参数被保留
    假如 用户在交互 args 提示中输入 {quoted_input}
    当 解析参数
    而且 那么解析后的参数向量包含各自独立的逻辑参数

  @req:r12
  场景: 未闭合引号被拒绝
    假如 用户在交互 args 提示中输入含未闭合引号的字符串
    当 解析参数
    而且 那么命令提示解析失败并拒绝执行

  @req:r12
  场景: 透传单个 flag
    假如 用户运行 llman x cc -- --version
    当 llman 执行 claude
    那么 claude 命令参数包含 --version

  @req:r12
  场景: 透传多参数并保持顺序
    假如 用户运行 llman x claude-code -- --message "hello world" --flag
    当 llman 执行 claude
    那么 claude 命令参数按顺序包含 --message、hello world 与 --flag
