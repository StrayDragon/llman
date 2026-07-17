# language: zh-CN
# 对应 spec: agent-tools-usage-stats — 输出含 table/json 与可选 TUI；table 用真实表格渲染无制表符；
# 颜色策略 auto（默认）/always/never，JSON 永不着色；路径默认缩写、--verbose 显示全路径。
功能: 输出格式、颜色策略与路径缩写
  @req:r2
  场景: JSON 输出可机读
    假如 用户运行 llman x codex stats --format json
    当 命令执行
    而且 那么stdout 为合法 JSON
    而且 而且含所选视图结果

  @req:r2
  场景: table 输出不含制表符
    假如 用户运行 llman x codex stats --format table
    当 命令执行
    而且 那么stdout 不含制表符

  @req:r2
  场景: NO_COLOR 在 auto 模式禁用 ANSI
    假如 用户运行 NO_COLOR=1 llman x cursor stats --format table --color auto
    当 命令执行
    而且 那么stdout 不含 ANSI 转义序列

  @req:r2
  场景: 非 TTY 在 auto 模式禁用 ANSI
    假如 用户运行 llman x claude-code stats --format table --color auto 重定向到文件
    当 命令执行
    而且 那么输出文件不含 ANSI 转义序列

  @req:r2
  场景: JSON 输出永不着色
    假如 用户运行 llman x codex stats --format json --color always
    当 命令执行
    而且 那么stdout 不含 ANSI 转义序列

  @req:r2
  场景: 默认隐藏完整绝对路径
    假如 用户运行 llman x cursor stats 且不带 --verbose
    当 命令执行
    而且 那么sessions 列表不打印完整绝对 cwd 路径

  @req:r2
  场景: 覆盖路径避免读取真实用户状态
    假如 用户用指向临时 fixture 的覆盖路径运行 stats
    当 命令执行
    而且 那么从 fixture 路径读取而非默认 home 目录
