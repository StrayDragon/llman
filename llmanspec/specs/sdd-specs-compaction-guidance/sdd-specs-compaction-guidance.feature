# language: zh-CN
# managed by llman sdd partition-migrate
功能: sdd-specs-compaction-guidance

  @req:r1
  场景: 当前版本不暴露 specs compact 子命令
    假如 用户执行 llman sdd --help
    当 查看帮助
    那么 帮助中不出现 specs compact 子命令

  @req:r1
  场景: 压缩决策以代码与 specs 为事实源
    假如 用户按新风格遵循 specs 压缩指引
    当 做 keep/merge/remove 决策
    那么 决策以代码与 specs 事实源为准

  @req:r1
  场景: 压缩流程包含压缩前后安全回归门
    假如 已准备好一份压缩方案
    当 执行压缩工作流
    那么 流程包含压缩前后输出的安全回归比对步骤

  @req:r1
  场景: update-skills 为各 tool 生成该技能
    假如 用户执行 llman sdd update-skills --no-interactive --all
    当 生成完成
    那么 各 tool 目标路径均生成 llman-sdd-specs-compact/SKILL.md

  @req:r1
  场景: 技能文本包含 freeze 建议
    假如 用户查看生成的 llman-sdd-specs-compact/SKILL.md
    当 阅读技能内容
    那么 文本明确建议在 archive 历史较大时先执行 llman sdd archive freeze --dry-run
