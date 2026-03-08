# Capability: codex-account-management

## MODIFIED Requirements

### Requirement: Provider 配置 upsert 到 codex config
切换组时，系统 MUST 将选中组的 provider 配置 upsert 到 `~/.codex/config.toml`，同时设置顶层 `model_provider = "<effective_name>"`。其中：

- `<effective_name>` 默认等于组名 `<group>`。
- 当存在 `[model_providers.<group>.llman_configs]` 且其中 `override_name` 为非空字符串时，`<effective_name>` MUST 使用该 `override_name`（而不是 `<group>`）。

同步写入时，系统 MUST：

- upsert 到 `[model_providers.<effective_name>]`（key 使用 `<effective_name>`）
- 将写入的 provider table 内 `name` 字段覆写为 `<effective_name>`
- 透传 provider table 的其它字段（例如 `request_max_retries`），但 MUST 排除 `.env` 与 `.llman_configs` 子表
- NOT 删除 codex config 中的其他已有配置

#### Scenario: 首次切换写入 provider（无 override_name）
- **WHEN** 用户选择 minimax 组且 `~/.codex/config.toml` 中无 `model_providers.minimax`
- **THEN** 系统将 minimax 的 provider 配置写入 codex config 并设置 `model_provider = "minimax"`

#### Scenario: 首次切换写入 provider（存在 override_name）
- **WHEN** 用户选择 b 组，且其配置包含 `[model_providers.b.llman_configs] override_name = "a"`，并且 `~/.codex/config.toml` 中无 `model_providers.a`
- **THEN** 系统将 b 的 provider 配置写入到 `model_providers.a`，同时设置 `model_provider = "a"`，且写入项中 `name = "a"`

#### Scenario: 透传额外字段且不写入 llman_configs
- **WHEN** 用户选择的 provider table 中包含额外字段（例如 `request_max_retries = 9999`）且存在 `override_name = "a"`
- **THEN** `~/.codex/config.toml` 的 `model_providers.a` 中保留该额外字段，且不包含 `llman_configs` 子表

#### Scenario: 重复切换不重复写入（基于 effective_name 幂等）
- **WHEN** 用户再次选择 b 组（其 `override_name = "a"`）且 codex config 中 `model_provider = "a"` 且 `model_providers.a` 已存在相同配置
- **THEN** 系统检测到配置已存在且一致，跳过写入

