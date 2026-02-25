## ADDED Requirements

### Requirement: SDD Specs 压缩治理技能
`llman sdd update-skills` MUST 生成 `llman-sdd-specs-compact` 技能，提供 specs 压缩治理流程。

该技能 MUST 覆盖：
- 现有 specs 盘点与重叠识别
- MUST/Scenario 冗余压缩策略
- 保留主干、迁移附录、归档建议
- 压缩后验证步骤与回归命令

#### Scenario: specs-compact 技能可生成
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --all`
- **THEN** Claude/Codex 目标路径均生成 `llman-sdd-specs-compact/SKILL.md`

### Requirement: CLI 入口预留（本次不实现）
SDD 规范 MUST 预留未来 `llman sdd specs compact` CLI 能力，但本次变更 MUST NOT 实现该命令。

#### Scenario: 当前版本不暴露 specs compact 子命令
- **WHEN** 用户执行 `llman sdd --help`
- **THEN** 帮助中不出现 `specs compact` 子命令
- **AND** 相关能力由 `llman-sdd-specs-compact` skill 承载
