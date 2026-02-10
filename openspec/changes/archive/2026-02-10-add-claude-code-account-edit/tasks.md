## 1. CLI 路由与 editor 复用

- [x] 1.1 为 Claude Code account 子命令增加 `Edit` action（`llman x claude-code account edit` / `llman x cc account edit`）
- [x] 1.2 在 `src/x/claude_code/command.rs` 实现 `handle_account_edit`：解析 config 路径、创建缺省文件、调用 editor、检查退出码
- [x] 1.3 抽取 Codex 现有 editor 选择/解析/执行逻辑到共享辅助模块，并让 Codex 与 Claude Code 共用（避免重复与漂移）

## 2. 默认模板与本地化文案

- [x] 2.1 新增 Claude Code 默认配置模板（例如 `templates/claude-code/default.toml`），至少包含 `[groups]`，并提供注释示例
- [x] 2.2 `locales/app.yml` 增加 `claude_code.account.config_created`、`claude_code.account.edited` 等成功提示
- [x] 2.3 `locales/app.yml` 增加 `claude_code.error.open_editor_failed`、`claude_code.error.invalid_editor_command`、`claude_code.error.editor_exit_status` 等错误提示
- [x] 2.4（可选但推荐）当 Claude Code 配置为空时，将 `no_configs_message()` 的建议中加入 `llman x claude-code account edit`

## 3. 测试覆盖

- [x] 3.1 增加集成测试：设置 `--config-dir` 指向临时目录，运行 `llman x claude-code account edit`，验证会创建 `claude-code.toml`
- [x] 3.2 增加集成测试：用临时可执行脚本作为 `$EDITOR`，验证“editor 参数 + config 路径作为最后一个参数追加”
- [x] 3.3 增加集成测试：editor 返回非零退出码时，命令失败且错误信息包含退出状态

## 4. 验证与回归检查

- [x] 4.1 运行 `just fmt` 与 `just test` 确保格式与测试通过
- [x] 4.2 手动 smoke：`llman x cc account edit`、`llman x claude-code account edit`、`llman x codex account edit`（确认两者行为一致且无回归）
