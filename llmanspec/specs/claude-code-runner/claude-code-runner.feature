# language: zh-CN
# managed by llman sdd partition-migrate
功能: claude-code-runner

  @req:r1
  场景: 引号参数被保留
    假如 用户在交互 args 提示中输入 {quoted_input}
    当 解析参数
    那么 解析后的参数向量包含各自独立的逻辑参数

  @req:r1
  场景: 未闭合引号被拒绝
    假如 用户在交互 args 提示中输入含未闭合引号的字符串
    当 解析参数
    那么 命令提示解析失败并拒绝执行

  @req:r1
  场景: 透传单个 flag
    假如 用户运行 llman x cc -- --version
    当 llman 执行 claude
    那么 claude 命令参数包含 --version

  @req:r1
  场景: 透传多参数并保持顺序
    假如 用户运行 llman x claude-code -- --message "hello world" --flag
    当 llman 执行 claude
    那么 claude 命令参数按顺序包含 --message、hello world 与 --flag

  @req:r1
  场景: 大写配置 pattern 也能匹配
    假如 配置中含危险 pattern {upper_pattern}
    而且 工具检查 {mixed_check}
    当 安全检测执行
    那么 该 pattern 被命中并输出安全警告

  @req:r1
  场景: 注入 PATH 危险键被拒绝
    假如 所选配置组含 PATH=/tmp/evil:$PATH
    当 用户运行 llman x cc -- --version
    那么 命令失败并报告危险环境变量被拒绝
    而且 未启动 claude

  @req:r1
  场景: 安全告警命中时中止执行
    假如 SecurityChecker 对当前 Claude settings 产生至少一条告警
    当 用户运行 llman x cc -- --version
    那么 stderr 含安全告警
    而且 命令非零退出
    而且 未启动 claude
