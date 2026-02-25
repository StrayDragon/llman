## ADDED Requirements

### Requirement: SDD 单文件归档冻结
`llman sdd archive freeze` MUST 将 `llmanspec/changes/archive/` 下符合规则的日期归档目录（`YYYY-MM-DD-*`）写入同一个冷备归档文件。

- 冷备文件路径 MUST 为 `llmanspec/changes/archive/freezed_changes.7z.archived`
- 命令每次执行 MUST 复用同一路径，不得为每次冻结创建独立归档文件
- 冻结完成后，被纳入本次冻结的源目录 MUST 从 `archive/` 下移除

#### Scenario: 首次冻结创建单文件归档
- **WHEN** 用户首次执行 `llman sdd archive freeze`
- **THEN** 创建 `freezed_changes.7z.archived`
- **AND** 本次冻结目录从 `archive/` 下移除

#### Scenario: 再次冻结写入同一归档文件
- **WHEN** 用户在后续执行 `llman sdd archive freeze`
- **THEN** 命令继续写入同一个 `freezed_changes.7z.archived`
- **AND** 不会创建新的独立归档文件或索引文件

#### Scenario: 逻辑追加不丢历史内容
- **WHEN** 冷备归档文件已存在且用户再次执行 `llman sdd archive freeze`
- **THEN** 新冻结内容与历史内容都能在后续 `llman sdd archive thaw` 中被恢复
- **AND** 归档文件路径仍保持 `freezed_changes.7z.archived`

#### Scenario: 冻结 dry-run
- **WHEN** 用户执行 `llman sdd archive freeze --dry-run`
- **THEN** 仅输出候选与目标归档文件路径
- **AND** 不写入归档文件且不删除源目录

#### Scenario: 冻结过滤
- **WHEN** 用户执行 `llman sdd archive freeze --before 2026-02-01 --keep-recent 2`
- **THEN** 仅冻结截止日期前且排除最近 N 条的候选目录

### Requirement: SDD 从单文件归档解冻
`llman sdd archive thaw` MUST 从 `freezed_changes.7z.archived` 恢复归档目录，支持全量恢复和按 change 选择恢复。

- 默认恢复目录 MUST 为 `llmanspec/changes/archive/.thawed/`
- 支持 `--change <id>` 选择性恢复
- 支持 `--dest <path>` 覆盖默认恢复目录

#### Scenario: 默认解冻到隔离目录
- **WHEN** 用户执行 `llman sdd archive thaw`
- **THEN** 解冻结果位于 `.thawed/`
- **AND** 主 `archive/` 根目录不被直接污染

#### Scenario: 选择性解冻
- **WHEN** 用户执行 `llman sdd archive thaw --change 2026-01-22-update-cli-quality-specs`
- **THEN** 仅恢复指定 change 目录

### Requirement: 单文件冻结流程安全性
冻结实现 MUST 使用安全写入策略，避免写入失败导致源目录丢失。

#### Scenario: 写入失败不删除源目录
- **WHEN** 冻结写入冷备归档文件失败
- **THEN** 命令返回非零
- **AND** 本次候选源归档目录保持原样
