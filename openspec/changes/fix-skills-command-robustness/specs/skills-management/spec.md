## ADDED Requirements
### Requirement: 断链 symlink 必须被视为已存在条目
在同步 targets 时，技能管理器 MUST 将断链 symlink 视为“已存在的文件系统条目”。即使 `exists()` 为 false，只要 `symlink_metadata()` 表明这是一个 symlink，也必须能够按冲突策略执行覆盖或移除。

#### Scenario: 覆盖断链 symlink
- **WHEN** 某 target 目录中存在名为 `<skill_id>` 的断链 symlink，且用户希望为该 target 启用该 skill
- **THEN** 冲突处理流程会运行，并可按选定冲突策略覆盖该断链 symlink

#### Scenario: 移除断链 symlink
- **WHEN** 某 target 目录中存在名为 `<skill_id>` 的断链 symlink，且用户希望为该 target 禁用该 skill
- **THEN** 该断链 symlink 会被移除

### Requirement: 冲突提示取消必须是安全 no-op
在交互模式下，如果用户取消冲突处理提示，管理器 MUST 将其视为安全的 abort/skip，并 MUST NOT 产生部分变更（包括不写入 registry）。

#### Scenario: 取消冲突提示
- **WHEN** target 存在冲突条目且用户在 overwrite/skip 提示中取消
- **THEN** 该 skill/target 对不应用任何变更，且命令在不写入 registry 的情况下退出

### Requirement: Registry 更新必须原子化
写入 `<skills_root>/registry.json` 时，管理器 MUST 采用原子写入方式，避免崩溃/中断导致 registry 损坏。

#### Scenario: registry 写入具备崩溃安全
- **WHEN** 管理器更新 `registry.json`
- **THEN** 磁盘上的文件要么是旧的有效 JSON，要么是新的有效 JSON，不得出现部分写入的损坏文件
