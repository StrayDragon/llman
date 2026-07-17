# language: zh-CN
# 对应 spec: claude-code-runner — 危险 pattern 匹配 MUST 大小写不敏感；环境变量注入 MUST
# 拒绝危险键（LD_PRELOAD/LD_LIBRARY_PATH/DYLD_*/PATH 及大小写变体），拒绝时报错且不启动子进程；
# SecurityChecker 发现告警时 MUST 中止执行不启动 claude。
功能: 危险模式匹配、环境变量注入与安全告警中止
  @req:r41
  场景: 大写配置 pattern 也能匹配
    假如 配置中含危险 pattern {upper_pattern}
    而且 而且工具检查 {mixed_check}
    当 安全检测执行
    而且 那么该 pattern 被命中并输出安全警告

  @req:r41
  场景: 注入 PATH 危险键被拒绝
    假如 所选配置组含 PATH=/tmp/evil:$PATH
    当 用户运行 llman x cc -- --version
    而且 那么命令失败并报告危险环境变量被拒绝
    而且 而且未启动 claude

  @req:r41
  场景: 安全告警命中时中止执行
    假如 SecurityChecker 对当前 Claude settings 产生至少一条告警
    当 用户运行 llman x cc -- --version
    那么 stderr 含安全告警
    而且 而且命令非零退出
    而且 而且未启动 claude
