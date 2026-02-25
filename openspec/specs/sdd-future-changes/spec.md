# sdd-future-changes Specification

## Purpose
TBD - created by archiving change upgrade-sdd-archive-freeze-and-structured-prompts. Update Purpose after archive.
## Requirements
### Requirement: Change 级 future 记录文件
llman SDD MUST 支持每个 change 的未来路线记录文件：`llmanspec/changes/<change-id>/future.md`。

该文件 SHOULD 采用统一章节结构：
- `## Deferred Items`
- `## Branch Options`
- `## Triggers to Reopen`
- `## Out of Scope for This Change`

#### Scenario: 新建 change 时可引导 future 文件
- **WHEN** 用户通过 `llman-sdd-new-change` 或 `llman-sdd-ff` 创建新变更
- **THEN** 技能说明中包含 future.md 的填写引导

#### Scenario: 持续推进时可补录 future
- **WHEN** 用户通过 `llman-sdd-continue` 推进变更
- **THEN** 技能允许并引导在 change 下补充 `future.md`

### Requirement: future 文件为可选且不阻塞归档
`future.md` MUST 是可选工件，不得阻塞 `llman sdd validate` 与 `llman sdd archive` 主流程。

#### Scenario: 无 future 文件仍可验证与归档
- **WHEN** change 目录中不存在 `future.md`
- **THEN** `llman sdd validate <change-id> --type change` 与 `llman sdd archive <change-id>` 不因缺失文件失败
