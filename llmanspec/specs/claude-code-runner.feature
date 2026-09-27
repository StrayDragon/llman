# language: zh-CN
# capability: claude-code-runner
# purpose: 规范 `llman x cc`（Claude Code runner）的交互参数解析、引号处理与执行行为合约。
# scope: llmanspec/specs/claude-code-runner.feature

功能: claude-code-runner

  @req:r12
  规则: 交互参数引号解析与 -- 参数透传
    对应 spec: claude-code-runner — llman x cc/claude-code run 交互模式收集参数 MUST 支持引号解析 （未闭合引号报错且不执行）；主命令 MUST 接受通过 -- 分隔的 trailing args 原样透传给 claude。

    场景: run-help-surface-accepts-trailing-args
      当 运行 llman x cc run --help
      那么 退出码为零
      那么 stdout 包含 --
  @req:r41
  规则: 危险模式匹配、环境变量注入与安全告警中止
    对应 spec: claude-code-runner — 危险 pattern 匹配 MUST 大小写不敏感；环境变量注入 MUST SecurityChecker 发现告警时 MUST 中止执行不启动 claude。

    场景: dangerous-env-aborts-before-launch
      假如 含配置组的 claude-code 配置
      当 以危险环境变量运行 llman x cc run
      那么 退出码非零
