## ADDED Requirements
### Requirement: change/spec ID 必须作为标识符处理而不是路径
所有接受 `change-id` 或 `spec-id` 的 `llman sdd` 子命令 MUST 将其视为标识符。实现 MUST 拒绝包含路径分隔符或穿越片段的值（例如：`/`、`\\`、`..`），并 MUST NOT 因此在 `llmanspec/` 之外执行任何文件系统操作。

#### Scenario: 拒绝路径穿越 ID
- **WHEN** 用户运行 `llman sdd archive ../oops`
- **THEN** 命令返回错误，且不会移动或修改任何文件

### Requirement: list 的冲突 flag 必须显式报错
`llman sdd list` MUST 将 `--specs` 与 `--changes` 视为互斥参数。若同时提供，两者冲突 MUST 返回非零错误并说明冲突原因。

#### Scenario: 同时传入 --specs 与 --changes
- **WHEN** 用户运行 `llman sdd list --specs --changes`
- **THEN** 命令返回错误并以非零退出

### Requirement: update-skills multi-tool 下的 --path 不得造成覆盖
当一次 `llman sdd update-skills` 生成多个 tool 的 skills 时，若仅提供单个 `--path` 覆盖路径，而实现无法保证不同 tool 的输出互不覆盖，则命令 MUST 以非零错误拒绝执行并给出安全用法提示。

#### Scenario: Multi-tool + --path 被拒绝
- **WHEN** 用户运行 `llman sdd update-skills --no-interactive --all --path ./skills-out`
- **THEN** 命令以非零退出并解释如何安全地按 tool 生成（避免覆盖）
