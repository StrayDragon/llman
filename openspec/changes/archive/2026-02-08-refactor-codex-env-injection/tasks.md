## 1. 配置层
- [x] 1.1 重写 `config.rs`：`ProviderConfig` 结构体（name/base_url/wire_api/env_key/env）、`Config` 加载保存、`upsert_to_codex_config()` 写入 `~/.codex/config.toml`

## 2. 命令层
- [x] 2.1 重写 `command.rs`：默认交互选组执行、`account edit`（首次使用模板）、`account import`、`run`

## 3. 交互层
- [x] 3.1 重写 `interactive.rs`：组选择、import 交互询问

## 4. 模板 + i18n
- [x] 4.1 创建 `templates/codex/default.toml` 模板文件
- [x] 4.2 更新 `locales/app.yml` codex 段

## 5. 验证
- [x] 5.1 `cargo +nightly fmt/clippy/test` 全部通过（141 tests）
