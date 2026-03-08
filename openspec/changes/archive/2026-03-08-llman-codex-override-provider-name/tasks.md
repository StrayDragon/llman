## 1. Config schema & parsing

- [x] 1.1 Extend `src/x/codex/config.rs` provider model to parse optional `[model_providers.<group>.llman_configs] override_name = "..."`
- [x] 1.2 Preserve provider table unknown keys (e.g., `request_max_retries`) during load/save, so `llman x codex account import` 不会静默丢字段

## 2. Codex config sync behavior

- [x] 2.1 Implement `effective_name = override_name.unwrap_or(group)` and validate `override_name` is not empty/blank
- [x] 2.2 Update provider→codex table building to:
  - include provider table extra fields
  - exclude `.env` / `.llman_configs`
  - force `name = effective_name`
- [x] 2.3 Update `upsert_to_codex_config` to upsert into `model_providers.<effective_name>` and set `model_provider = "<effective_name>"`, including idempotency checks based on `effective_name`

## 3. Docs & templates

- [x] 3.1 Update `templates/codex/default.toml` to document `llman_configs.override_name` and show an example

## 4. Tests

- [x] 4.1 Add unit tests for provider→codex table generation (extra fields preserved; `.env` / `.llman_configs` excluded; `name` overridden)
- [x] 4.2 Add a test for `upsert_to_codex_config` using `tempfile::TempDir` + `crate::test_utils::TestProcess` to set `HOME`, verifying:
  - override writes under `model_providers.<override_name>`
  - repeat sync is idempotent (no rewrite when unchanged)
