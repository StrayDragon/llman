# language: zh-CN
# 对应 spec: claude-code-account-management — account env <GROUP> 输出经 shell 安全引号转义的
# 注入语句（POSIX/PowerShell），键名升序输出并经安全校验；缺失配置或组名报错；
# account list 展示敏感环境变量值时 MUST 脱敏。
功能: account env 注入输出与 account list 敏感值脱敏
  @req:r2
  场景: 非 Windows 输出 POSIX export
    假如 用户在非 Windows 平台运行 llman x claude-code account env {group}
    而且 而且该组含 FOO=bar
    当 命令执行
    那么 stdout 含 export FOO='bar'

  @req:r2
  场景: Windows 输出 PowerShell env
    假如 用户在 Windows 运行 llman x claude-code account env {group}
    而且 而且该组含 FOO=bar
    当 命令执行
    那么 stdout 含 $env:FOO='bar'

  @req:r2
  场景: 键名升序稳定输出
    假如 组含 B=2 与 A=1
    当 命令执行
    那么 stdout 行按 A 然后 B 顺序输出

  @req:r2
  场景: 非法键名时报错且不输出注入语句
    假如 组含非法键名 BAD-KEY=1
    当 命令执行
    而且 那么非零退出
    而且 而且stdout 不含注入语句

  @req:r2
  场景: 组不存在时报错
    假如 用户运行 llman x claude-code account env {missing_group}
    当 命令执行
    而且 那么非零退出并报告该组不存在

  @req:r2
  场景: account list 对敏感值脱敏
    假如 配置组含 DB_PASSWORD={secret}
    当 用户运行 llman x claude-code account list
    而且 那么输出中该值被脱敏
    而且 而且不包含完整明文
