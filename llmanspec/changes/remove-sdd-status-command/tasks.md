# Tasks

Seam（测试边界，全部复用既有 harness，无新接缝）：
CLI 子进程（`tests/it/` 集成套件 + smoke 列表）、模板门禁（`just check-sdd-templates`）、
SDD 校验（`llman sdd validate`）。

## T1: 移除 CLI 命令面与直属测试

- [ ] 删除 `crates/llman-sdd/src/sdd/commands/status.rs`；移除
  `commands/mod.rs` 的模块声明与 `command.rs:243` 枚举变体、`command.rs:830-834`
  分发臂。
- [ ] 移除 `src/bin/llmanspec.rs` 中 `llmanspec status` ≡ `llman sdd status`
  的别名映射（含 doc 注释行）。
- [ ] `tests/it/sdd_bdd_compat.rs:197` smoke 列表移除 `&["sdd", "status"]`。
- [ ] `locales/app.yml`：删除 status 命令专属 key 段
  （no_changes_dir / no_active_changes / changes_header / no_specs /
  specs_header / no_tasks_status / complete_status / just_now 等，以实际归属
  段为准）；1462 行 frontmatter 未知字段错误文案中 `run \`llman sdd status\``
  改为 `run \`llman sdd show\`/\`llman sdd list\``。
- 验证：`cargo build` 通过；`llman sdd status` 报未知命令非零退出；
  `cargo nextest run`（it + bdd 相关子集）绿。
  Seam：CLI 子进程。

## T2: 模板与文档收敛查询路径

- [ ] `templates/sdd/{en,zh-Hans}/skills/{draft,propose,apply-cycle}.md`
  移除 `llman sdd status` 引用，替换为 `show` / `list` 等价指引。
- [ ] `llmanspec/AGENTS.md` 两处（frontmatter SSOT 表下方与正文写作约束）
  `llman sdd status` 查询指引改为 `llman sdd show` / `llman sdd list`。
- [ ] 根 `AGENTS.md` 生命周期表格中「用 `show`/`status --json` 查」改为
  `show --json`。
- [ ] `llman sdd init --update` resync 渲染产物（`.agents/skills/**`）。
- 验证：`just check-sdd-templates` 绿；全库 grep `sdd status` 仅剩
  T3 评估项。Seam：模板门禁。

## T3: 残留清扫与全门禁

- [ ] 评估 `tests/bdd_steps.rs:487` spec.toon 迁移 fixture 中的
  `run llman sdd status` 字样：纯历史数据则保留原样并记录；与行为耦合则改写。
- [ ] 全库 grep `sdd status` 归零核查（排除 archive 历史与本文档）。
- [ ] 全门禁：`just test`、`just check-sdd-templates`、
  `llman sdd validate --all --strict --no-interactive --no-check`。
- 验证：全绿。Seam：CLI 子进程 + 模板门禁 + SDD 校验。
  [blocked-by: T1, T2]
