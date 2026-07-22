# Tasks: Enhance SDD with mattpocock.capabilities

> 按 P0→P1→P2→P3 依赖顺序。每项是垂直切片。`[blocked-by]` 标记依赖。

## Seams under test

- [x] seam: BDD-on Partitioned SSOT 链路（`validate --strict` + `.feature` `@req`）(confirmed)
- [x] seam: skill 模板渲染（`init --update` + `metadata.llman_sdd` 一致性）(confirmed)

## P0：最高 ROI（先验证模式）

- [x] task-1: verify 双轴审查 — 修改 `.agents/skills/llman-sdd-verify/SKILL.md` 加 Standards 轴（Fowler 12 smell baseline + AGENTS.md 权威优先） [blocked-by: none]
- [x] task-2: propose seam 确认 + 垂直切片 — 修改 `.agents/skills/llman-sdd-propose/SKILL.md`：tasks 写前加 seam 声明段；tasks 支 `[blocked-by]` [blocked-by: none]
- [x] task-3: P0 校验 — `just check` + `llman sdd validate --all --strict --no-interactive` [blocked-by: task-1, task-2]

## P1：pipeline 核心阶段增强

- [x] task-4: explore grilling 深对齐 — 修改 `.agents/skills/llman-sdd-explore/SKILL.md` 加 grilling 分支（逐问 + 推荐答案 + 回写 proposal） [blocked-by: task-3]
- [x] task-5: apply diagnose 紧反馈 — 修改 `.agents/skills/llman-sdd-apply/SKILL.md` 加失败升级 diagnose 子流程（red-capable 命令门禁） [blocked-by: task-3]
- [x] task-6: P1 校验 — `just check` + `llman sdd validate --all --strict --no-interactive` [blocked-by: task-4, task-5]

## P2：独立可选 skill

- [x] task-7: 新增 `llman-sdd-arch-review` skill（model-invoked，metadata.skill_set=optional） [blocked-by: task-6]
- [x] task-8: 新增 `llman-sdd-wayfinder` skill（user-invoked，disable-model-invocation） [blocked-by: task-6]
- [x] task-9: 新增 `llman-sdd-research` skill（model-invoked，metadata.skill_set=optional） [blocked-by: task-6]
- [x] task-10: config.yaml extra_skills — 分析后决定不改（OPTIONAL_SKILL_NAMES 编译期限定，需后续 CLI change）；限制已记入 design.md [blocked-by: task-7, task-9]
- [x] task-11: P2 校验 — `just check` + `just check-sdd-templates` + `llman sdd validate --all --strict --no-interactive` [blocked-by: task-10]

## P3：领域语言治理 + 路由文档

- [x] task-12: 领域语言回写 spec.toon — explore/grilling 加 sharpening 行为（更新 requirement statement，不建 CONTEXT.md） [blocked-by: task-11]
- [x] task-13: AGENTS.md 增强路由 — SDD 段加"可选增强能力"小节，索引 P0-P2 触发条件 + 固化 seam/depth 词汇定义 [blocked-by: task-11]
- [x] task-14: P3 校验 — `just check`（540 passed）+ `check-sdd-templates`（passed）+ sdd validate（32 passed）；注：`doc-check` 失败于 `src/sdd/command.rs:457` 既有 `<CHANGE>` rustdoc lint（非本 change 引入，单独修） [blocked-by: task-12, task-13]

## 收尾

- [x] task-15: full mode 验证 — `cargo test --features bdd`（rstest-bdd harness 全绿） [blocked-by: task-14]
- [x] task-16: `llman sdd change finalize enhance-sdd-with-mattpocock-capabilities` [blocked-by: task-15]
