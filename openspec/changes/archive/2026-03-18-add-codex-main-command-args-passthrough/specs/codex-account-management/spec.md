# Capability: codex-account-management

## ADDED Requirements

### Requirement: 主命令支持 `-- <codex-args...>` 透传
当用户运行 `llman x codex` 的主命令路径（无子命令）时，系统 MUST 支持 `-- <codex-args...>` 形式，并在用户完成交互选择 provider 后执行：

- 同步 provider 到 codex config（按既有逻辑）
- 注入 provider env（按既有逻辑）
- 执行 `codex <codex-args...>`

系统 MUST 将 `--` 之后的每个 argv token 作为一个独立参数传给 `codex`，且 MUST 不对这些参数进行解析或重写。

#### Scenario: 透传 codex 参数
- **WHEN** 用户运行 `llman x codex -- --help -m o3` 并在交互中选择任意 provider
- **THEN** 系统执行 `codex --help -m o3`

#### Scenario: 未提供透传参数
- **WHEN** 用户运行 `llman x codex` 并在交互中选择任意 provider
- **THEN** 系统执行 `codex`

