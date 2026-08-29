# Tasks: add-sdd-pipeline-guards

> 前置：Specs landing 已完成（3 个 capability 的新规则在绑定分支落地）。
> 每 task 垂直切片：实现 → BDD 场景/门禁 → 独立可验证。收尾单 commit 由 finalize 负责。

- [x] T1 `valid_scope` 检查下沉 validate
  `crates/llman-sdd`：validate --specs/--all/--strict 路径按 D4 语义校验 scope 路径存在性；新增 fixture Given（失效 scope 项目）；驱动 sdd-workflow 新 @executable 场景；跑 `cargo test --features bdd` + `just check`。验证：新场景绿 + 既有 validate 场景不回归。
  [blocked-by: specs-landing]

- [x] T2 `change diff` commitCount 与 finalize/checkpoint 多 commit 提示
  `crates/llman-sdd`：diff 新增 `--json`（含 commitCount 数值键）与人读计数行；finalize/checkpoint 计数 > 1 打印不阻断提示（D2）；驱动 sdd-workflow 新 @executable 场景（0-commit fixture 断言 commitCount 为数字）。验证：场景绿。
  [blocked-by: specs-landing]

- [x] T3 `list` idleDays 与 draft/designed 停留标注
  `crates/llman-sdd`：list --json 每 change 增 `idleDays`（D3 口径）；文本人读对 draft/designed 追加标注；`tests/bdd_steps.rs` 数值路径步骤支持数组段；驱动 sdd-workflow 新 @executable 场景。验证：场景绿。
  [blocked-by: specs-landing]

- [x] T4 review 三时点接线进模板
  `templates/sdd/{en,zh-Hans}/skills/llman-sdd-{apply,verify,archive}.md`：按新规则插入 review 检查点（apply 批次后 / verify finalize 前 / archive 逐个前，非零退出 = CRITICAL → 停止修复）；`init --update` resync；根 `AGENTS.md` 增强表「双轴审查」行对齐 r103（D5）。验证：`just check-sdd-templates` 绿 + 渲染产物 grep 命中。
  [blocked-by: specs-landing]

- [x] T5 模板单元职责与引用合规
  校验修复单元从 graph/quick 摘除（保留五个 spec 编辑类 skill）；apply 的 arch-review 引用补 fallback 措辞（r96 合规）；apply-cycle 重试预算与 apply 统一表述；`init --update` resync。验证：`just check-sdd-templates` 绿 + 渲染产物 grep。
  [blocked-by: specs-landing]

- [x] T6 verify 编号连续性修复 + 门禁断言
  verify 模板组装：阶段守卫单元移出宿主有序列表内部（D7）；`just check-sdd-templates` 脚本新增渲染产物步骤编号连续性断言。验证：断言绿且 verify 渲染产物 1→2→…连续。
  [blocked-by: T4]

- [x] T7 本项目启用 arch-review
  本项目 `config.yaml` `extra_skills: [llman-sdd-arch-review]` + `init --update`。验证：`.agents/skills/llman-sdd-arch-review/` 存在且 validate --all 绿。
  [blocked-by: T5]

- T8 全量验证与收尾
  `cargo test --features bdd` + `just check` + `just check-sdd-templates` + `llman sdd review`（本 change 自身走一次三时点检查点）+ 建议 `llman-sdd-verify`。
  [blocked-by: T1, T2, T3, T4, T5, T6, T7]
