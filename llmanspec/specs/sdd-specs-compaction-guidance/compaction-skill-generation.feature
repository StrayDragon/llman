# language: zh-CN
# 对应 spec: sdd-specs-compaction-guidance — llman sdd update-skills MUST 生成
# llman-sdd-specs-compact 技能，提供 specs 压缩治理流程；且在 archive 历史噪声较大时
# 建议先执行 freeze。
功能: specs 压缩治理技能可生成且含 freeze 建议
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
