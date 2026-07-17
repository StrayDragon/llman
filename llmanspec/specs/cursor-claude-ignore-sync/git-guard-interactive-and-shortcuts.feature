# language: zh-CN
# 对应 spec: cursor-claude-ignore-sync — 系统 MUST 强制检查 git root（非 git 目录报错，
# --force 可绕过）；MUST 提供交互式模式选择 targets/预览/确认；SHOULD 通过 llman x 子命令
# 提供快捷方式（cc→claude-shared，cursor→cursor）。
功能: git 守卫、交互式模式与 x 子命令快捷方式
  @req:r19
  场景: 非 git 目录报错
    假如 当前目录向上遍历找不到 .git
    当 用户执行 llman tool sync-ignore
    那么 系统报错并返回非零退出码

  @req:r19
  场景: --force 允许在非 git 目录运行
    假如 当前目录向上遍历找不到 .git
    当 用户执行 llman tool sync-ignore --force
    那么 系统将当前目录视为 root 并继续执行
    而且 而且仍默认 dry-run

  @req:r19
  场景: 交互式多选 targets 并反选删除提示
    假如 用户执行 llman tool sync-ignore --interactive
    当 系统显示界面
    那么 显示 MultiSelect 列表（含 .ignore / .cursorignore / .claude/settings.json / .claude/settings.local.json）
    而且 而且标注 exists/missing

  @req:r19
  场景: 交互式预览与确认
    假如 用户在交互模式下完成 targets 选择
    当 系统展示计划
    那么 显示每个 target 的 create/update/unchanged/delete 计划与规则数量

  @req:r19
  场景: 通过 cc 子命令同步到 Claude Code
    假如 用户执行 llman x cc sync-ignore
    当 系统写入
    那么 默认将 targets 限制为 claude-shared（.claude/settings.json）

  @req:r19
  场景: 通过 cursor 子命令同步到 Cursor
    假如 用户执行 llman x cursor sync-ignore
    当 系统写入
    那么 默认将 targets 限制为 cursor（.cursorignore）
