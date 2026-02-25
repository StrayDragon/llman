## 1. Archive 子命令面与兼容路由

- [x] 1.1 在 `src/sdd/command.rs` 将 `archive` 重构为子命令组，新增 `run/freeze/thaw` 路由并保留 `archive <change-id>` 兼容入口。
- [x] 1.2 在 `src/sdd/change/archive.rs` 拆分现有执行路径为 `run` 语义，确保 `--skip-specs`、`--dry-run`、隐藏 `--force` 与旧行为一致。
- [x] 1.3 为 `llman sdd --help` 与 `llman sdd archive --help` 补充/更新断言测试，验证帮助文本与兼容入口说明正确。

## 2. 冻结与解冻引擎（单文件 7z 冷备）

- [x] 2.1 新增冻结模块（如 `src/sdd/change/freeze.rs`），实现候选归档扫描：仅匹配 `llmanspec/changes/archive/YYYY-MM-DD-*`。
- [x] 2.2 在 `Cargo.toml` 引入 `sevenz-rust2`，并封装 7z 压缩/解压能力供 freeze/thaw 使用。
- [x] 2.3 实现 `archive freeze`：将候选目录写入同一归档文件 `llmanspec/changes/archive/freezed_changes.7z.archived`（已有归档时执行逻辑追加并原子替换）。
- [x] 2.4 实现 `archive freeze --dry-run` 与过滤参数（`--before`、`--keep-recent`），保证 dry-run 无写入。
- [x] 2.5 实现 `archive thaw`，默认解冻到 `.thawed/`，支持 `--change` 与 `--dest`。
- [x] 2.6 增加失败安全：归档文件写入失败时不删除源归档目录（通过临时文件 + 原子 rename）。
- [x] 2.7 新增回归测试验证“二次 freeze 后 thaw 仍包含首次冻结内容”（逻辑追加不丢历史）。

## 3. future.md 结构与流程接入

- [x] 3.1 为 `llmanspec/changes/<id>/future.md` 定义模板结构（Deferred/Branch/Triggers/Out-of-scope）。
- [x] 3.2 在相关 SDD 模板（new/ff/continue/explore）加入 future.md 引导，确保“可选但建议维护”的语义。
- [x] 3.3 如引入校验逻辑，确保 `future.md` 缺失不会导致 `validate`/`archive` 失败，并添加对应测试。

## 4. skills 结构化提示规范升级

- [x] 4.1 更新 `templates/sdd/{en,zh-Hans}/skills/*.md` 与 `spec-driven/*.md`，统一为 Context/Goal/Constraints/Workflow/Decision Policy/Output Contract 结构。
- [x] 4.2 新增 `llman-sdd-specs-compact` 双语技能模板，并在 `src/sdd/project/templates.rs` 的 skill 列表与 embedded mapping注册。
- [x] 4.3 确保模板内容不包含对外部技能的硬依赖表达（尤其是“先调用某外部技能再执行”）。
- [x] 4.4 运行并修复 `just check-sdd-templates`，确保版本头、locale 对齐与 region 展开合法。

## 5. 集成测试与回归验证

- [x] 5.1 在 `tests/sdd_integration_tests.rs` 新增 freeze/thaw 主流程测试：创建归档目录 -> freeze -> 验证单文件归档 -> thaw。
- [x] 5.2 新增 freeze dry-run 与失败安全测试：确认 dry-run 不写入、异常场景不删除源目录。
- [x] 5.3 新增 `update-skills` 回归测试，验证生成 `llman-sdd-specs-compact/SKILL.md` 且不出现外部技能硬依赖字样。
- [x] 5.4 新增 future.md 相关行为测试（引导或可选性），验证缺失 future.md 时 validate/archive 仍可通过。
- [x] 5.5 全量回归执行：`just test`、`just check-sdd-templates`，记录失败项并修复至通过。

## 6. 规范同步与验收闭环

- [x] 6.1 更新 `openspec/specs/sdd-workflow/spec.md` 主规范，使其覆盖 archive 子命令组、freeze/thaw、future.md、结构化提示、specs-compact 技能。
- [x] 6.2 在实现完成后执行 `openspec validate upgrade-sdd-archive-freeze-and-structured-prompts --type change --strict --no-interactive` 并修复不一致。
- [x] 6.3 形成验收记录：列出新增命令、测试证据、兼容性结果（`archive <id>` 仍可用）。
