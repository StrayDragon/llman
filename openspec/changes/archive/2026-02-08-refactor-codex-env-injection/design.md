## Context

Codex CLI 从 `~/.codex/config.toml` 读取 `model_provider` 和 `[model_providers.<name>]`，其中 `env_key` 指定读取 API Key 的环境变量名。llman 可以同时：
1. 将 provider 配置 upsert 到 codex 的 config.toml
2. 在启动 codex 时注入 env_key 对应的实际值

## Goals / Non-Goals

- Goals: 统一 codex/claude-code 多账号切换体验；provider 配置自动同步到 codex config
- Non-Goals: 不管理 codex config 中 provider 以外的字段

## Decisions

- **llman config 格式** (`~/.config/llman/codex.toml`):
  ```toml
  [model_providers.minimax]
  name = "minimax"
  base_url = "https://minimax/codex/v1"
  wire_api = "responses"
  env_key = "MINIMAX_CODEX_API_KEY"

  [model_providers.minimax.env]
  MINIMAX_CODEX_API_KEY = "sk-xxxxx"
  ```

- **upsert 策略**: 读取 `~/.codex/config.toml` 为 `toml::Value`，设置 `model_provider = "<name>"` 和 `model_providers.<name>` = provider 字段（不含 env），写回。只更新不删除其他内容。

- **import 交互**: 询问 group_name、base_url、env_key_id（默认 CODEX_API_KEY）、api_key_value，生成配置写入 codex.toml。

## Risks / Trade-offs

- 写入 `~/.codex/config.toml` 有覆盖用户自定义配置的风险 → 仅 upsert model_providers 和 model_provider 字段

## Open Questions

- 无
