## MODIFIED Requirements

### Requirement: SDD 命令范围
`llman sdd` MUST 暴露以下命令集合：`init`、`update`、`update-skills`、`list`、`show`、`validate`、`archive`、`import`、`export`。  
其中 `import` 与 `export` MUST 作为 `llmanspec` 与外部规范目录互转的唯一入口。实现 MUST NOT 暴露 `migrate --from/--to` 兼容别名。  
在 SDD 子命令组中 MUST 不提供 `change`、`spec`、`view`、`completion`、`config` 等额外子命令。

#### Scenario: 帮助文本包含 import/export
- **WHEN** 用户执行 `llman sdd --help`
- **THEN** 帮助文本包含 `import` 与 `export`
- **AND** 帮助文本不包含 `migrate`

#### Scenario: style 参数强约束
- **WHEN** 用户执行 `llman sdd import` 或 `llman sdd export` 且缺少 `--style`
- **THEN** 命令返回非零并提示 `--style openspec` 为必填

#### Scenario: 旧命名不可用
- **WHEN** 用户执行 `llman sdd migrate --from openspec`
- **THEN** 命令返回未知子命令错误
