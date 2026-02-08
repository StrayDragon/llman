## Why

当前 `llman x codex` 的账号组切换采用符号链接方式直接替换 `~/.codex/config.toml`，侵入性强且与 `llman x claude-code` 风格不一致。Codex CLI 支持 `[model_providers.<name>]` 中的 `env_key` 字段来指定 API Key 环境变量名，可通过环境变量注入 + provider 配置 upsert 实现多供应商切换。

## What Changes

- **BREAKING** 移除旧的符号链接 + groups 目录 + metadata.toml 方案
- 新增 `~/.config/llman/codex.toml`，格式为 `[model_providers.<name>]` + `[model_providers.<name>.env]`
- 切换组时：将 `[model_providers.<name>]`（不含 `.env`）upsert 到 `~/.codex/config.toml`，同时注入 `.env` 中的环境变量到 codex 进程
- CLI 入口：
  - `llman x codex`：交互选组 → upsert provider → 注入 env → 执行 codex
  - `llman x codex run`：`--group` 非交互 / `-i` 交互模式
  - `llman x codex account`：默认 edit
  - `llman x codex account edit`：编辑器打开 `codex.toml`
  - `llman x codex account import`：交互式创建新的 provider 组
- 移除 `account list/create/use/delete`、`templates/codex/*.toml`

## Impact

- 受影响的规范：`codex-account-management`
- 受影响的代码：`src/x/codex/`（全部重写）、`locales/app.yml`（codex 段）
