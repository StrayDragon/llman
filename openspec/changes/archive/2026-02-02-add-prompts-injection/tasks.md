## 1. Specs
- [x] 1.1 新增 `prompts-management` 规范增量（别名、app 列表、注入路径、冲突策略）。
- [x] 1.2 运行 `openspec validate add-prompts-injection --strict --no-interactive` 并修复问题。

## 2. Implementation (after approval)
- [x] 2.1 将 `prompts` 作为文档/提示默认命令名，保留 `prompt` 兼容别名（更新 `locales/app.yml` 与 README 示例）。
- [x] 2.2 扩展 prompt app 列表：新增 `codex`、`claude-code`（含别名/兼容命名的解析策略）。
- [x] 2.3 为 `codex` 实现注入到 prompts 目录（`--scope user|project|all`），并确保只写入顶层文件（不创建子目录）。
- [x] 2.4 为 `claude-code` 实现注入到 memory file（`--scope user|project|all`）的托管块更新（可重复运行、保留非托管内容）。
- [x] 2.5 统一覆盖/冲突策略：交互确认；非交互模式需 `--force`（或等价旗标）方可覆盖。
- [x] 2.6 增加/更新单元测试与集成测试覆盖（路径解析、扩展名映射、托管块更新）。
- [x] 2.7 运行 `just test`（或 `cargo +nightly test --all`）与必要的手动 smoke check（cursor/codex/claude-code）。
