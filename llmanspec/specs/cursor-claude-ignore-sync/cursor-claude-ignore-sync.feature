# language: zh-CN
# managed by llman sdd partition-migrate
功能: cursor-claude-ignore-sync

  @req:r1
  场景: 非 git 目录报错
    假如 当前目录向上遍历找不到 .git
    当 用户执行 llman tool sync-ignore
    那么 系统报错并返回非零退出码

  @req:r1
  场景: --force 允许在非 git 目录运行
    假如 当前目录向上遍历找不到 .git
    当 用户执行 llman tool sync-ignore --force
    那么 系统将当前目录视为 root 并继续执行
    而且 仍默认 dry-run

  @req:r1
  场景: 交互式多选 targets 并反选删除提示
    假如 用户执行 llman tool sync-ignore --interactive
    当 系统显示界面
    那么 显示 MultiSelect 列表（含 .ignore / .cursorignore / .claude/settings.json / .claude/settings.local.json）
    而且 标注 exists/missing

  @req:r1
  场景: 交互式预览与确认
    假如 用户在交互模式下完成 targets 选择
    当 系统展示计划
    那么 显示每个 target 的 create/update/unchanged/delete 计划与规则数量

  @req:r1
  场景: 通过 cc 子命令同步到 Claude Code
    假如 用户执行 llman x cc sync-ignore
    当 系统写入
    那么 默认将 targets 限制为 claude-shared（.claude/settings.json）

  @req:r1
  场景: 通过 cursor 子命令同步到 Cursor
    假如 用户执行 llman x cursor sync-ignore
    当 系统写入
    那么 默认将 targets 限制为 cursor（.cursorignore）

  @req:r1
  场景: include 规则被正确识别
    假如 .ignore 内容包含 !{pattern}
    当 系统解析 .ignore
    那么 必须把 {pattern} 记录为 include 规则
    而且 不是 ignore 规则

  @req:r1
  场景: 写回时稳定输出顺序
    假如 系统写回 .ignore 或 .cursorignore
    当 写回完成
    那么 先输出所有 ignore
    而且 再输出所有 include（以 ! 前缀）

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
    而且 不得删除

  @req:r1
  场景: 默认 dry-run 预览不写入
    假如 当前目录位于一个 git repo 内
    当 用户执行 llman tool sync-ignore
    那么 系统自动发现项目内存在的 sources
    而且 默认仅预览不写入

  @req:r1
  场景: --yes 应用写入并自动创建缺失 targets
    假如 当前目录位于一个 git repo 内
    当 用户执行 llman tool sync-ignore --yes
    那么 系统把 union 结果写入/创建默认 targets
    而且 默认 targets 含 .ignore、.cursorignore、.claude/settings.json

  @req:r1
  场景: --target 限制输出目标
    假如 用户执行 llman tool sync-ignore --target {target}
    当 系统写入
    那么 仅写入/创建该 target 对应文件
