## Context

`llman x codex` 目前通过 `codex.toml` 中的 `[model_providers.<group>]` 读取 provider 配置，并在执行 codex 前将该 provider upsert 到 `~/.codex/config.toml`（同时注入 `[model_providers.<group>.env]` 中的环境变量）。

现状中，llman 的 `<group>` 会直接用于 Codex config 的 key 与 `name` 字段，导致当用户希望：

- 复用同一组名（或需要改名），但希望 Codex 历史记录连续；
- 或者希望 llman 的组名与 Codex provider identity 解耦；

时无法做到。

此外，同步实现如果仅写入固定字段集合，会造成 provider table 的其它字段在同步到 Codex config 时丢失（例如用户自定义/上游支持的 `request_max_retries` 等）。

## Goals / Non-Goals

**Goals:**
- 支持在 llman 的 provider 配置中声明 `llman_configs.override_name`，用于覆写写入 Codex config 的 provider key 与 `name` 字段。
- 同步到 `~/.codex/config.toml` 时保留 provider table 的其它字段（除明确排除的 `.env` / `.llman_configs`）。
- 保持行为向后兼容：未配置 `override_name` 时行为不变。
- 保持幂等：当 Codex config 已经是目标状态时不重复写入。

**Non-Goals:**
- 不尝试自动清理由历史写入导致的 `model_providers.<old_name>` 残留项（避免破坏用户已有配置；也与“不可删除其他配置”的既有约束一致）。
- 不改变 `llman x codex` 的交互界面与组选择逻辑（仍以 llman 的 `<group>` 为选择维度）。

## Decisions

1) **新增 llman 扩展段：`llman_configs`**
- 在 `codex.toml` 的 provider table 下新增可选子表 `[model_providers.<group>.llman_configs]`。
- 字段：`override_name: Option<String>`。
- 该子表只影响“写入 Codex config”阶段，不影响组选择与 env 注入。

2) **计算 effective provider key**
- 约定 `effective_key = override_name.unwrap_or(group_name)`。
- `effective_key` 用于：
  - `~/.codex/config.toml` 中 `[model_providers.<effective_key>]` 的 key
  - `model_provider = "<effective_key>"`
  - provider table 内 `name = "<effective_key>"`

3) **透传 provider 额外字段**
- ProviderConfig 增加 `extra` 容器（例如 `#[serde(flatten)] HashMap<String, toml::Value>`），用于保留未显式建模的 provider table 字段（例如 `request_max_retries`）。
- 构造用于 upsert 的 TOML table 时：
  - 从已建模字段与 `extra` 合并生成（保持 key/value 不变）
  - 明确排除 `env` 与 `llman_configs`
  - 最后强制覆写 `name = effective_key`（确保 Codex config 内一致性）

4) **幂等比较基于 effective_key**
- 判断“已是目标状态”时，必须比较：
  - `model_provider == effective_key`
  - `model_providers.<effective_key>` 与新生成 table 完全一致
- 仅当上述均成立时才跳过写入，避免出现配置 key 变更但仍被判定“已同步”的情况。

## Risks / Trade-offs

- **[Risk] override_name 冲突覆盖已有 Codex provider** → **Mitigation**：这是显式覆写行为；文档中提示用户确保 `override_name` 唯一且有意义。
- **[Risk] 留下旧 key 的残留 provider entry** → **Mitigation**：不自动删除，遵守“不删除其他配置”的原则；需要时用户可手动清理。
- **[Risk] 透传额外字段导致与 Codex 上游字段语义不匹配** → **Mitigation**：llman 仅做“结构性同步”，不解释字段语义；保留字段优于静默丢失。

## Migration Plan

- 纯增量变更：旧 `codex.toml` 无需修改即可继续工作。
- 需要稳定/唯一 Codex provider identity 的用户可选择性添加：
  - `[model_providers.<group>.llman_configs]`
  - `override_name = "<unique-name>"`

## Open Questions

- `override_name` 是否需要做格式校验（例如禁止空字符串、或限制字符集）？初版至少应拒绝空/全空白值并给出明确错误。

