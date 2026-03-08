## Why

目前 `llman x codex` 会将选中组的 `group_name` 同时作为：

- llman 的组名（用于交互选择与 `--group <name>`）
- 同步到 `~/.codex/config.toml` 的 `[model_providers.<name>]` key
- 同步到 `~/.codex/config.toml` 的 `model_provider = "<name>"`
- provider table 内的 `name = "<name>"`

但在实际使用中，Codex 的会话/历史记录与 provider 名称强相关：当 provider 名称复用、或需要在不同 llman 组名之间切换时，会出现“历史记录找不到/被混淆”的问题。

我们需要一种方式：llman 继续用自己的组名管理与选择配置，但在写入 Codex config 时可以使用一个稳定且唯一的 provider 名称。

## What Changes

- 在 llman 的 `codex.toml` provider 配置中新增一个可选扩展段：
  - `[model_providers.<group>.llman_configs]`
  - `override_name = "<codex_provider_name>"`
- 当 `override_name` 存在时，同步到 `~/.codex/config.toml` 的效果变为：
  - upsert 写入到 `[model_providers.<override_name>]`（而不是 `<group>`）
  - provider table 内 `name = "<override_name>"`（覆写）
  - 顶层 `model_provider = "<override_name>"`（覆写）
- 当 `override_name` 不存在时，行为保持不变（使用 `<group>` 作为写入 key 与 name）。
- 同步到 Codex config 时：
  - MUST 排除 `.env` 子表（现有行为）
  - MUST 排除 `.llman_configs` 子表（新扩展段，仅供 llman 使用）
  - SHOULD 保留 provider table 的其它字段（例如 `request_max_retries` 等自定义/上游字段），避免“只同步子集字段”导致配置丢失

## Capabilities

### New Capabilities
- （none）

### Modified Capabilities
- `codex-account-management`: provider 配置同步逻辑支持 `llman_configs.override_name`，并确保同步时不丢失 provider table 的其它字段（除 `.env` / `.llman_configs`）。

## Impact

- Code: `src/x/codex/config.rs`（解析新字段、计算写入 key、构造 upsert table、幂等比较）
- Templates: `templates/codex/default.toml`（补充 `llman_configs.override_name` 示例与说明）
- Tests: 增加覆盖：
  - 存在 `override_name` 时写入 key 与 `name` 覆写
  - 幂等（配置一致时不重复写入）
  - provider table 额外字段透传（且不写入 `.env` / `.llman_configs`）

