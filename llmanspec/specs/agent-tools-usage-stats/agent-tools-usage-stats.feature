# language: zh-CN
# managed by llman sdd partition-migrate
功能: agent-tools-usage-stats

  @req:r1
  场景: 命令帮助可发现
    假如 用户运行 llman x codex stats --help
    当 命令执行
    那么 打印 stats 命令帮助文本并成功退出

  @req:r1
  场景: 默认过滤到当前目录
    假如 用户在目录 /p/a 运行 llman x claude-code stats
    当 命令执行
    那么 输出排除 cwd 不精确等于 /p/a 的会话

  @req:r1
  场景: 默认视图为 summary
    假如 用户运行 llman x cursor stats 且不带 --view
    当 命令执行
    那么 渲染 summary 视图

  @req:r1
  场景: sessions 视图 id 可用于下钻
    假如 用户运行 llman x codex stats --view sessions 并看到含 id X 的行
    当 用户运行 llman x codex stats --view session --id X
    那么 可查看该会话详情

  @req:r1
  场景: session 视图必须提供 id
    假如 用户运行 llman x claude-code stats --view session 且不带 --id
    当 命令执行
    那么 返回错误并提示需提供 --id

  @req:r1
  场景: group-by month 按日历月聚合
    假如 用户运行 llman x codex stats --view trend --group-by month
    当 命令执行
    那么 输出含过滤数据集中每个日历月一个桶

  @req:r1
  场景: sessions 视图每个会话一行
    假如 用户运行 llman x cursor stats --view sessions
    当 命令执行
    那么 每行恰好对应一个 Cursor composer 会话

  @req:r1
  场景: RFC3339 createdAt 被接受
    假如 某 Cursor bubble 记录 createdAt 为 RFC3339 字符串
    当 计算会话时间戳
    那么 使用解析后的时间

  @req:r1
  场景: since 过滤掉旧会话
    假如 用户运行 llman x codex stats --since {since_time}
    当 命令执行
    那么 结束于该时间戳之前的会话被排除

  @req:r1
  场景: 仅日期 until 包含整天
    假如 用户运行 llman x cursor stats --until {date_only}
    当 命令执行
    那么 该日期本地时间结束的会话被包含

  @req:r1
  场景: last 与 since/until 互斥
    假如 用户运行 llman x claude-code stats --last 7d --since {since_time}
    当 命令执行
    那么 返回错误指明 flag 不兼容

  @req:r1
  场景: limit 仅作用于 sessions 视图
    假如 用户运行 llman x cursor stats --view sessions --limit 10
    当 命令执行
    那么 sessions 视图返回 10 个会话

  @req:r1
  场景: JSON 输出可机读
    假如 用户运行 llman x codex stats --format json
    当 命令执行
    那么 stdout 为合法 JSON
    而且 含所选视图结果

  @req:r1
  场景: table 输出不含制表符
    假如 用户运行 llman x codex stats --format table
    当 命令执行
    那么 stdout 不含制表符

  @req:r1
  场景: NO_COLOR 在 auto 模式禁用 ANSI
    假如 用户运行 NO_COLOR=1 llman x cursor stats --format table --color auto
    当 命令执行
    那么 stdout 不含 ANSI 转义序列

  @req:r1
  场景: 非 TTY 在 auto 模式禁用 ANSI
    假如 用户运行 llman x claude-code stats --format table --color auto 重定向到文件
    当 命令执行
    那么 输出文件不含 ANSI 转义序列

  @req:r1
  场景: JSON 输出永不着色
    假如 用户运行 llman x codex stats --format json --color always
    当 命令执行
    那么 stdout 不含 ANSI 转义序列

  @req:r1
  场景: 默认隐藏完整绝对路径
    假如 用户运行 llman x cursor stats 且不带 --verbose
    当 命令执行
    那么 sessions 列表不打印完整绝对 cwd 路径

  @req:r1
  场景: 覆盖路径避免读取真实用户状态
    假如 用户用指向临时 fixture 的覆盖路径运行 stats
    当 命令执行
    那么 从 fixture 路径读取而非默认 home 目录

  @req:r1
  场景: 未知 token 不破坏聚合
    假如 过滤数据集中部分会话无 token 信息
    当 命令执行
    那么 仍成功渲染所请求视图

  @req:r1
  场景: 离线执行
    假如 机器无网络连接
    当 用户运行 llman x codex stats
    那么 仍用本地状态运行

  @req:r1
  场景: sidechain 可见且计入总量
    假如 某 Claude Code 项目含主会话与带 token 的关联 sidechain
    当 查看 sessions 视图
    那么 列出两个会话

  @req:r1
  场景: 覆盖率可见
    假如 数据集含缺失 token 信息的会话
    当 生成报告
    那么 含表示 known-token 覆盖率的字段

  @req:r1
  场景: 默认仅用 thread tokens_used
    假如 用户运行 llman x codex stats 且不带 breakdown 选项
    当 命令执行
    那么 不要求解析 rollout JSONL 即可产出总量

  @req:r1
  场景: no-sidechain 禁用 sidechain
    假如 用户运行 llman x claude-code stats --no-sidechain
    当 命令执行
    那么 sidechain 会话被排除出所有视图

  @req:r1
  场景: codex breakdown 在 TUI 显示进度
    假如 用户运行 llman x codex stats --tui --with-breakdown 且解析多个 rollout 文件
    当 扫描执行
    那么 TUI 显示持续更新的进度指示器直到完成

  @req:r1
  场景: TUI 过滤更新视图
    假如 用户在 TUI 过滤表单修改时间范围并提交
    当 重新扫描
    那么 显示的 sessions/trend 更新以反映新范围
