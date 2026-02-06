# claude-code-runner Specification

## Purpose
TBD - created by archiving change fix-claude-code-command-args-and-security. Update Purpose after archive.
## Requirements
### Requirement: 交互 args 解析必须支持引号
当 `llman x claude-code run` 在交互模式下收集参数字符串时，它 MUST 支持引号解析，使单个逻辑参数可以包含空格。若参数字符串无法按该规则解析（例如未闭合引号），命令 MUST 报错且 MUST NOT 执行。

#### Scenario: 引号参数被保留
- **WHEN** 用户在交互 args 提示中输入 `--message "hello world" --flag`
- **THEN** 解析后的参数向量包含 `--message`、`hello world` 与 `--flag`

#### Scenario: 未闭合引号会被拒绝
- **WHEN** 用户在交互 args 提示中输入 `--message "hello`
- **THEN** 命令提示解析失败并拒绝执行

### Requirement: 危险模式匹配必须大小写不敏感
安全警告检测 MUST 将危险 patterns 视为大小写不敏感（包括配置文件中提供的 patterns）。

#### Scenario: 大写配置 pattern 也能匹配
- **WHEN** 配置中包含危险 pattern `RM -RF`，且工具检查 `Bash(rm -rf /tmp/x)`
- **THEN** 该 pattern 被命中并输出安全警告
