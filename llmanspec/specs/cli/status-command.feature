# language: zh-CN
# 对应 spec: cli — status 命令输出纯 TOON（kind llman.sdd.status），含 counts/changes/tasks/
# ops 与 next；target 解析顺序：精确匹配 > 唯一模糊匹配 > 多匹配汇总 > 无匹配报错；
# 仅显示未完成任务与 pending ops；按 c<N>- 优先级前缀排序；--json 兼容。
功能: status 命令 TOON 输出与 target 解析
  @req:r2
  场景: 无参数输出项目级 TOON
    假如 用户运行 llman sdd status
    当 命令执行
    而且 那么输出为 kind llman.sdd.status 的 TOON
    而且 而且含 counts{}、按优先级排序的 changes[] 与 next 字段

  @req:r2
  场景: 指定 change 名输出单变更 TOON
    假如 用户运行 llman sdd status {change}
    当 命令执行
    而且 那么输出含 change{} 与 tasks[]（仅未完成）与 next

  @req:r2
  场景: 指定归档 change 输出归档 TOON
    假如 用户运行 llman sdd status {archived_change}
    当 命令执行
    而且 那么输出含 change{status=archived}、ops[] 与 next

  @req:r2
  场景: --json 等价 --format json
    假如 status --json 与 status --format json 输出相同
    当 用户运行 llman sdd status --json
    而且 那么输出为 JSON
    而且 而且内容与 --format json 一致

  @req:r2
  场景: 模糊匹配唯一时输出单变更
    假如 用户运行 llman sdd status {fuzzy}（仅一个匹配）
    当 命令执行
    而且 那么输出单变更 TOON（change{} + ops[]）

  @req:r2
  场景: 多匹配时输出汇总
    假如 用户运行 llman sdd status {date_prefix}（匹配多个归档）
    当 命令执行
    而且 那么输出汇总 TOON
    而且 而且changes[] 列出全部匹配

  @req:r2
  场景: 无匹配时报错
    假如 用户运行 llman sdd status {nonexistent}
    当 命令执行
    而且 那么命令报错并给出最近建议列表

  @req:r2
  场景: 仅显示未完成任务
    假如 用户运行 llman sdd status {change}（3/5 任务完成）
    当 命令执行
    而且 那么tasks[] 仅含未完成任务
    而且 而且不含全部 5 个

  @req:r2
  场景: 按 cN 前缀优先级排序
    假如 项目含 c2-foo 与 c1-bar
    当 用户运行 llman sdd status
    那么 TOON changes[] 先列 c1-bar 再列 c2-foo

  @req:r2
  场景: 单变更 JSON 含详细字段
    假如 用户运行 llman sdd status --format json {change}
    当 命令执行
    那么 JSON 输出含 change 名、stage、completedTasks、totalTasks、nextAction
