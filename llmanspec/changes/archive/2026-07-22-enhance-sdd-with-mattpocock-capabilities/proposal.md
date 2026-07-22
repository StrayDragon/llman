---
depends_on: []
branch: feat/enhance-sdd-with-mattpocock-capabilities
base_sha: 9d4e3558ce06855906ce2a087c024aeff34420f1
checkpointed: true
checkpoint_sha: 9d4e3558ce06855906ce2a087c024aeff34420f1
---

# Proposal: Enhance SDD with mattpocock.capabilities (方案 B：能力内化)

## Why

经过对 `mattpocock.skills`（git submodule 参考）的能力分析，发现 llman SDD 当前 pipeline **强在"合约的可执行性与闭环门禁"**（BDD-on Partitioned SSOT、`validate --strict`、attach/checkpoint），但**弱在"对齐的深度与设计的词汇"**：

- `explore` 的"问 1-3 个问题"过浅，缺乏 mattpocock `grilling` 那种"逐问走决策树 + 附推荐答案"的强制对齐。
- `propose` 的 `tasks.md` 是平铺清单，无 `to-spec`/`tdd` 的 **seam 前置确认**与 `to-tickets` 的 **tracer-bullet 垂直切片 + blocking edges**。
- `apply` 失败时的自修复是 ad-hoc 的，缺少 `diagnosing-bugs` 的 **紧反馈循环（red-capable 命令）** 纪律。
- `verify` 是单轴（artifacts vs code），缺少 `code-review` 的 **双轴分离（Standards + Spec 并行）**。
- 缺少 `improve-codebase-architecture`（主动发现 shallow module）、`wayfinder`（大型雾状工作）、`research`（后台文献委托）等独立能力。

本变更采用**方案 B（能力内化）**：把 mattpocock 的高价值能力内核**作为可选增强**织进 llman SDD 现有 5 阶段（explore→propose→apply→verify→archive），并用 llman 已有的单 SSOT（`spec.toon`）替代 mattpocock 的 `CONTEXT.md`，避免双权威。不照抄 skill 文件，而是提炼能力并以 llman 的词汇与门禁重写。

## What Changes

### P0（最高 ROI，先验证模式）

- **propose seam 确认 + 垂直切片 tasks**（r101）：`tasks.md` 写之前 MUST 先列出将测试的 seam（来自 `*.feature` GWT 步骤）并确认；tasks 按 tracer-bullet 垂直切片组织，支持 `[blocked-by: <task>]` 依赖标记。
- **verify 双轴审查**（r103）：verify 报告 MUST 含 **Spec 轴**（现有：实现 vs `spec.toon` MUST/SHALL + `.feature` GWT）与 **Standards 轴**（新增：`AGENTS.md` coding style + Fowler 12 smell baseline）分离呈现。

### P1（pipeline 核心阶段增强）

- **explore grilling 深对齐模式**（r100）：explore MUST 支持可选 grilling 分支——逐问走决策树、每问附推荐答案、能查到的事实（读 spec/代码）不问用户、决策回写 proposal。
- **apply diagnose 紧反馈循环**（r102）：apply 自修复失败时 MUST 升级到 diagnose 子流程——先建 red-capable 命令（紧反馈循环）再假设，而非直接猜测修复。

### P2（独立可选 skill，不进线性 pipeline）

- **llman-sdd-arch-review**（r104）：model-invoked，扫描 shallow module 产出 deepening 候选；触发词含"架构审查"/"deepening"。
- **llman-sdd-wayfinder**（r105）：user-invoked，大型雾状工作规划为 decision ticket map；与 llman `change` + `graph` 结合。
- **llman-sdd-research**（r106）：model-invoked，后台 agent 委托外部文献调研；产出回写 proposal 的 Further Notes。

### P3（领域语言治理 + 路由文档）

- **领域语言治理回写 spec.toon**（r107）：grilling/explore 中 sharpening 模糊术语时 MUST 更新 `spec.toon` requirement statement，MUST NOT 另建 `CONTEXT.md` glossary 第二权威。
- **AGENTS.md 增强路由**（r108）：AGENTS.md 的 SDD 段 MUST 含"可选增强能力"小节，索引 P0-P2 的增强触发条件。

## Capabilities

| Capability | Change | Type |
|------------|--------|------|
| `sdd-workflow` | explore/propose/apply/verify pipeline 增强（grilling/seam/diagnose/dual-axis） | modify |
| `skills-management` | 新增 3 个独立 skill（arch-review/wayfinder/research） | add |

## Design Decisions

见 `design.md`。核心三条：
1. **单 SSOT**：领域语义回 `spec.toon`，不引入 `CONTEXT.md`。
2. **渐进式**：每项增强是 pipeline 阶段的可选分支，默认行为不变。
3. **先纯 SKILL.md prompt，后 CLI**：验证模式稳定后再把高频操作固化成 `llman sdd` 子命令。

## Impact

- **Skills**: 修改 4 个现有 SKILL.md（explore/propose/apply/verify）；新增 3 个 skill 目录（arch-review/wayfinder/research）。
- **AGENTS.md**: SDD 段新增"可选增强能力"小节。
- **CLI**: 本 change 不新增子命令（先 prompt 驱动）。
- **config.yaml**: 新增的 3 个独立 skill 若要被 `init --update` 管理，需纳入 `extra_skills`（wayfinder 为 user-invoked 可不放；arch-review/research 为 model-invoked 建议放）。
- **Breakage**: 无。所有增强为可选分支，默认 pipeline 行为不变。
- **mattpocock.skills submodule**: 保留作上游参考，不进运行产物。

## Open Questions

无（探索阶段已通过 AskUserQuestion 确认方案 B / P0-P3 全量 / 先纯 SKILL.md / 保留 submodule）。
