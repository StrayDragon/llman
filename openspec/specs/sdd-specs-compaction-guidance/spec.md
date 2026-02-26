# sdd-specs-compaction-guidance Specification

## Purpose
TBD - created by archiving change upgrade-sdd-archive-freeze-and-structured-prompts. Update Purpose after archive.
## Requirements
### Requirement: SDD Specs 压缩治理技能
`llman sdd update-skills` MUST 生成 `llman-sdd-specs-compact` 技能，提供 specs 压缩治理流程。

该技能 MUST 覆盖：
- 现有 specs 盘点与重叠识别
- MUST/Scenario 冗余压缩策略
- 保留主干、迁移附录、归档建议
- 在 archive 历史噪声较大时先执行 freeze 建议（`llman sdd archive freeze --dry-run` 与执行命令）
- 压缩后验证步骤与回归命令

#### Scenario: specs-compact 技能可生成
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --all`
- **THEN** Claude/Codex 目标路径均生成 `llman-sdd-specs-compact/SKILL.md`

#### Scenario: specs-compact 包含 freeze 建议
- **WHEN** 用户查看生成的 `llman-sdd-specs-compact/SKILL.md`
- **THEN** 文本明确建议在 archive 历史较大时先执行 `llman sdd archive freeze --dry-run`

### Requirement: CLI 入口预留（本次不实现）
SDD 规范 MUST 预留未来 `llman sdd specs compact` CLI 能力，但本次变更 MUST NOT 实现该命令。

#### Scenario: 当前版本不暴露 specs compact 子命令
- **WHEN** 用户执行 `llman sdd --help`
- **THEN** 帮助中不出现 `specs compact` 子命令
- **AND** 相关能力由 `llman-sdd-specs-compact` skill 承载

### Requirement: Compaction Guidance Must Reference ISON Source of Truth
Compaction guidance in new style MUST define ISON source artifacts as canonical for compaction decisions.

#### Scenario: Compaction flow references canonical ISON source
- **WHEN** a user follows specs compaction guidance in new style
- **THEN** keep/merge/remove decisions are derived from ISON source artifacts
- **AND** rendered Markdown is treated as a compatibility surface

### Requirement: Compaction Guidance Includes Safety Regression Check
Compaction guidance MUST include a safety regression comparison step between baseline and compacted outputs.

#### Scenario: Compaction includes safety check gate
- **WHEN** a compaction plan is prepared
- **THEN** the workflow includes a before/after safety check gate
- **AND** warns users to stop when safety-critical behavior changes
