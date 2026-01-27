## ADDED Requirements
### Requirement: SDD 归档前置校验
`llman sdd archive` MUST 在修改 specs 或移动 change 目录之前，对本次涉及的 specs 执行与 `llman sdd validate --strict --no-interactive` 等价的校验（包括 frontmatter 与 staleness）。归档校验 MUST 以重建后的 spec 内容为准，并在 staleness 判断中将本次归档涉及的 spec 视为已更新。任何 Error 或 Warn MUST 阻止归档并返回非零。

#### Scenario: 校验失败阻止归档
- **WHEN** 用户执行 `llman sdd archive <change-id>` 且任一 spec 校验失败
- **THEN** 命令退出非零，且不写入/移动任何文件

#### Scenario: staleness 警告视为失败
- **WHEN** staleness 状态为 `STALE` 或 `WARN`
- **THEN** 归档失败并提示修复

#### Scenario: 允许强制绕过
- **WHEN** 用户执行 `llman sdd archive <change-id> --force`
- **THEN** 归档继续执行即使校验失败

#### Scenario: force 参数隐藏
- **WHEN** 用户执行 `llman sdd archive --help`
- **THEN** 帮助输出不包含 `--force`

#### Scenario: skip-specs 跳过校验
- **WHEN** 用户执行 `llman sdd archive <change-id> --skip-specs`
- **THEN** 不执行归档前的 spec 校验

#### Scenario: 错误提示不引导绕过
- **WHEN** 归档因校验失败而中止
- **THEN** 输出仅提示修复校验问题，不提示 `--force`

### Requirement: SDD skills 输出符合 Agent Skills SKILL.md 规范
`llman sdd update-skills` MUST 生成符合 Agent Skills 规范的 `SKILL.md` frontmatter，至少包含 `name` 与 `description`：
- `name` MUST 与技能目录名一致，且仅包含小写字母/数字/连字符、长度 1-64、不得以连字符开头/结尾、不得包含连续连字符。
- `description` MUST 为 1-1024 字符的非空描述文本。
- `license`、`compatibility`、`metadata`、`allowed-tools` MAY 在需要时提供。

#### Scenario: name 与目录一致
- **WHEN** `llman sdd update-skills` 写入 `llman-sdd-archive/SKILL.md`
- **THEN** frontmatter `name` 为 `llman-sdd-archive`

#### Scenario: description 非空
- **WHEN** `llman sdd update-skills` 生成任意 SKILL.md
- **THEN** frontmatter `description` 为非空字符串

### Requirement: SDD skills 不暴露绕过参数
`llman sdd update-skills` 生成的 skills 内容 MUST 不包含 `--force` 绕过提示或示例。

#### Scenario: skills 不包含 --force
- **WHEN** 维护者运行 `llman sdd update-skills`
- **THEN** 生成的 SKILL.md 不提及 `--force`
