# language: zh-CN
# 对应 spec: agent-tools-usage-stats — 三工具（codex/claude-code/cursor）均提供 stats 子命令；
# v1 仅含当前工作目录记录；支持 summary/trend/sessions/session 视图；sessions 提供稳定 id；
# trend 支持 day/week/month 聚合；支持时间范围过滤与 sessions 排序/limit。
功能: stats 命令视图、稳定 id 与过滤
  场景: 命令帮助可发现
    假如 用户运行 llman x codex stats --help
    当 命令执行
    那么打印 stats 命令帮助文本并成功退出

  场景: 默认过滤到当前目录
    假如 用户在目录 /p/a 运行 llman x claude-code stats
    当 命令执行
    那么输出排除 cwd 不精确等于 /p/a 的会话

  场景: 默认视图为 summary
    假如 用户运行 llman x cursor stats 且不带 --view
    当 命令执行
    那么渲染 summary 视图

  场景: sessions 视图 id 可用于下钻
    假如 用户运行 llman x codex stats --view sessions 并看到含 id X 的行
    当 用户运行 llman x codex stats --view session --id X
    那么可查看该会话详情

  场景: session 视图必须提供 id
    假如 用户运行 llman x claude-code stats --view session 且不带 --id
    当 命令执行
    那么返回错误并提示需提供 --id

  场景: group-by month 按日历月聚合
    假如 用户运行 llman x codex stats --view trend --group-by month
    当 命令执行
    那么输出含过滤数据集中每个日历月一个桶

  场景: sessions 视图每个会话一行
    假如 用户运行 llman x cursor stats --view sessions
    当 命令执行
    那么每行恰好对应一个 Cursor composer 会话

  场景: RFC3339 createdAt 被接受
    假如 某 Cursor bubble 记录 createdAt 为 RFC3339 字符串
    当 计算会话时间戳
    那么使用解析后的时间

  场景: since 过滤掉旧会话
    假如 用户运行 llman x codex stats --since {since_time}
    当 命令执行
    那么结束于该时间戳之前的会话被排除

  场景: 仅日期 until 包含整天
    假如 用户运行 llman x cursor stats --until {date_only}
    当 命令执行
    那么该日期本地时间结束的会话被包含

  场景: last 与 since/until 互斥
    假如 用户运行 llman x claude-code stats --last 7d --since {since_time}
    当 命令执行
    那么返回错误指明 flag 不兼容

  场景: limit 仅作用于 sessions 视图
    假如 用户运行 llman x cursor stats --view sessions --limit 10
    当 命令执行
    那么sessions 视图返回 10 个会话
