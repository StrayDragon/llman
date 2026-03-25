## ADDED Requirements

### Requirement: 主命令必须支持 `--` 参数透传给 claude
当用户运行 `llman x cc`（或 `llman x claude-code`）且未指定子命令时，命令 MUST 接受通过 `--` 分隔的 trailing args，并在完成配置组选择与环境变量注入后，将该参数向量按顺序原样传递给底层 `claude` 命令执行。

#### Scenario: 透传单个 flag
- **WHEN** 用户运行 `llman x cc -- --version`
- **THEN** llman 执行的 `claude` 命令参数包含 `--version`

#### Scenario: 透传多参数并保持顺序
- **WHEN** 用户运行 `llman x claude-code -- --message "hello world" --flag`
- **THEN** llman 执行的 `claude` 命令参数按顺序包含 `--message`、`hello world` 与 `--flag`
