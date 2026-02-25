## MODIFIED Requirements

### Requirement: SDD 归档流程
`llman sdd archive` MUST 支持子命令组工作流，并保持兼容入口可用：
- 兼容入口：`llman sdd archive <change-id>`（等价于 `llman sdd archive run <change-id>`）
- 标准入口：`llman sdd archive run <change-id>`

`archive run` MUST 延续当前行为：将 delta 合并到 `llmanspec/specs`，并将 change 移动到 `llmanspec/changes/archive/YYYY-MM-DD-<change-id>`；`--skip-specs`、`--dry-run`、隐藏 `--force` 行为 MUST 保持一致。

#### Scenario: 兼容入口仍可归档
- **WHEN** 用户执行 `llman sdd archive add-sample`
- **THEN** 命令成功归档并与 `llman sdd archive run add-sample` 结果一致

#### Scenario: run 子命令行为与历史一致
- **WHEN** 用户执行 `llman sdd archive run <change-id> --skip-specs`
- **THEN** 仅移动目录到 archive 且不修改主 specs

#### Scenario: force 仍保持隐藏
- **WHEN** 用户执行 `llman sdd archive --help` 或 `llman sdd archive run --help`
- **THEN** 帮助文本不显示 `--force`

### Requirement: SDD Skills 生成与更新
`llman sdd update-skills` 生成的 workflow skills 集 MUST 新增 `llman-sdd-specs-compact`，并在现有 `llman-sdd-*` skills 中使用统一结构化提示协议（见 `sdd-structured-skill-prompts` capability）。

#### Scenario: 生成包含 specs-compact skill
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --tool codex`
- **THEN** 目标路径下存在 `llman-sdd-specs-compact/SKILL.md`

#### Scenario: 结构化协议在已生成技能中可见
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --all`
- **THEN** 生成的 `llman-sdd-archive` 与 `llman-sdd-explore` 等技能均包含统一结构化章节
