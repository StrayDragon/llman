## 1. CLI 脚手架与模板
- [ ] 1.1 在 `src/cli.rs` 增加 `SddArgs` 与 `SddCommands`，新增 `src/sdd/mod.rs` 作为命令入口。
- [ ] 1.2 添加 spec-driven 模板（proposal/spec/design/tasks），存放为资源文件或嵌入式字符串。
- [ ] 1.3 实现 `llman sdd init`：创建 `openspec/` 结构并写入 `openspec/AGENTS.md`（含 LLMAN-SDD 受管提示块）。

## 2. 核心 SDD 命令
- [ ] 2.1 实现 `llman sdd update`：刷新指令与模板，仅更新 LLMAN-SDD 受管块且不触碰 specs/changes 内容。
- [ ] 2.2 实现 `llman sdd list`（默认 changes，`--specs` 列 specs）并提供 `--json` 输出。
- [ ] 2.3 实现 `llman sdd show <id> --type change|spec`：原样输出 markdown，并提供 `--json`。
- [ ] 2.4 实现 `llman sdd validate <id>`：严格校验并支持 `--strict --no-interactive --json`。

## 3. 归档流程
- [ ] 3.1 实现 ADDED/MODIFIED/REMOVED/RENAMED delta 解析（`openspec/changes/<id>/specs/**/spec.md`）。
- [ ] 3.2 实现 `llman sdd archive <id>`：合并 delta 到 `openspec/specs`，再移动到 `openspec/changes/archive/YYYY-MM-DD-<id>`。
- [ ] 3.3 支持 `--skip-specs`（仅归档目录，不更新 specs）。
- [ ] 3.4 支持 `--dry-run` 预览（输出将变更的文件列表与目标路径）。

## 4. 消息与文档
- [ ] 4.1 在 `locales/app.yml` 增加 SDD 相关提示与错误文案，并通过 `t!` 输出。
- [ ] 4.2 更新 `README.md`，补充 `llman sdd` 用法与示例。

## 5. 测试与验证
- [ ] 5.1 为校验与归档合并逻辑添加单元测试（含 dry-run 与 json 输出场景）。
- [ ] 5.2 添加 init/list/show/validate/archive 的集成测试（临时目录）。
- [ ] 5.3 验证：`just fmt`、`just lint`、`just test`，以及手动 smoke（如 `llman sdd init`、`llman sdd list --specs`、`llman sdd validate <id> --strict --no-interactive`）。
