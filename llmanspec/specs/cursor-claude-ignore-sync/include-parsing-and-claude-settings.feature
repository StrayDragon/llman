# language: zh-CN
# 对应 spec: cursor-claude-ignore-sync — 系统 MUST 解析 gitignore 风格的 include（!pattern）
# 规则并稳定写回；MUST 解析/更新 Claude Code settings（仅 permissions.deny 的 Read(...)），
# 尽量保留 JSONC 注释（best-effort），并保留非 Read deny 规则。
功能: include 规则解析与 Claude Code settings 读写策略
  @req:r1
  场景: include 规则被正确识别
    假如 .ignore 内容包含 !{pattern}
    当 系统解析 .ignore
    那么 必须把 {pattern} 记录为 include 规则
    而且 而且不是 ignore 规则

  @req:r1
  场景: 写回时稳定输出顺序
    假如 系统写回 .ignore 或 .cursorignore
    当 写回完成
    那么 先输出所有 ignore
    而且 而且再输出所有 include（以 ! 前缀）

  @req:r1
  场景: 仅转换 permissions.deny 的 Read
    假如 .claude/settings.json 的 permissions.deny 含 Read(./{glob})
    当 系统解析 Claude Code settings
    那么 必须提取 {glob} 作为 ignore 规则

  @req:r1
  场景: include 规则无法写入 Claude Code 时告警并跳过
    假如 union 结果含至少一条 include（如 !{pattern}）
    当 系统写入 .claude/settings.json
    那么 必须跳过 include 规则

  @req:r1
  场景: 保留非 Read deny 规则
    假如 .claude/settings.json 的 permissions.deny 含非 Read 项（如 WebFetch(...)）
    当 系统写入 Claude Code settings
    那么 必须保留这些非 Read 项
    而且 而且不得删除
