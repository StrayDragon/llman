---
change_id: add-sdd-bdd-partitioned-ssot
title: "BDD Partitioned SSOT：feature harness + toon 约束 + sdd 自管理（发布级一步到位）"
status: proposal
priority: 1300
author: agent
track: SDD
---

# add-sdd-bdd-partitioned-ssot

## Why

BDD-on 下可执行 GWT 双写在 `spec.toon` 与 `.feature`：token 近似翻倍，且「文字合约」无法被项目测试引擎真实 harness。下游（如 xylitol）与 llman 本仓都需要一版**最优、可试用反馈**的模型：

1. **`.feature` = harness 层**：可执行场景唯一正文，由 `bdd.run_command` / `validate --check` 真实跑绿。
2. **`spec.toon` = 约束层**：requirements + 不可执行 scenarios；给 context/propose 强约束，不靠 agent 自觉。
3. **`llman sdd` 自管理**：validate / list / show / context / delta / archive / migrate 把两种形态当一等公民。

旧 solidify「toon 投影生成 feature」与本次目标冲突，必须在**同一发布**内翻转，并提供迁移命令让下游一次适配。

## What Changes

### A) Partitioned SSOT 合约

| 层 | 权威 | 内容 |
|---|---|---|
| 约束 | `spec.toon` | `requirements` + **不可执行** scenarios（完整 GWT） |
| Harness | `*.feature` | 可执行场景唯一 GWT；`@req:<req_id>`（或等价标签）挂回 requirement |
| 禁止 | — | 同一 scenario id 的 GWT 同时完整存在于 toon 与 feature |

BDD-off：**行为不变**。

### B) validate 门禁

BDD-on 时 MUST：

- `@req:X` → toon 存在 requirement `X`（缺失 = CRITICAL）
- 同一 id 的可执行 GWT 双写 = CRITICAL（`--strict`）；非 strict 至少 WARNING
- 不可执行 toon scenario id 不得出现在 `.feature`
- `--check` 仍执行 `bdd.run_command`（harness 真跑）

### C) list / show 形态展示

- `list --specs` / `--json`：`morphology` 字段（`constraintsReqCount`、`harnessScenarioCount`、`reqLinkCoverage`、`dualWriteCount`）
- `show <spec>`：分段 **Constraints** + **Harness**；JSON 同构；禁止把 feature 全文再从 toon 打一遍

### D) solidify 语义翻转

| 旧 | 新 |
|---|---|
| 把 delta `feature:true` GWT 投影写入 `.feature` | **一致性门禁**（链接 / 双写 / 标签）；可选 `--write-stubs` 仅补缺失场景骨架与 `@req`，不覆盖已有 GWT 正文 |
| BDD-off no-op | 保持 no-op |

### E) feature_delta + archive 双管道

- Change 可携带 `specs/<cap>/<cap>.feature.delta.toon`（或约定文件名）：按 scenario id 的 `add` / `modify` / `remove`（整块替换，非行 diff）
- Requirement / 不可执行 scenario：仍用现有 toon `ops` / `op_scenarios`
- `archive`：merge toon ops **且** apply feature_delta 到主 `.feature`
- 废弃「archive 整文件覆盖复制 feature 导致冲突」路径中与 Partitioned 冲突的部分

### F) context / index：单次 embed

- 可执行 GWT：**feature 优先**；toon 侧若仍残留同 id 可执行行，index **不得**再 embed 第二份正文
- `compute_spec_hash` 仍包含 `.feature`
- 修改 `sdd-context` 中「toon wins on collision / 合并加总」类合约为 Partitioned 语义

### G) 迁移与 dogfood

- `llman sdd project partition-migrate [--dry-run]`：把 `feature:true` 双写拆成「toon 去可执行 GWT + feature 补 `@req`」
- **本仓 30 个 spec 全部迁完**并 `validate --all --strict` + 抽样 `--check` 绿
- 发布说明：下游破坏性适配清单（xylitol 等）

### H) Skills

propose / apply / solidify / archive / verify / explore：删除「toon 唯一真源、feature 只读衍生」；改为 Partitioned + feature_delta 引导。

## Capabilities

- `sdd-bdd-mode-compat`
- `sdd-workflow`
- `sdd-context`
- `sdd-structured-skill-prompts`

## Impact

- **破坏性发布**：BDD-on 下游必须跑 `partition-migrate`（或等价手工）后再依赖新 validate
- solidify / archive / context embed / propose skill 行为均变
- ethics.risk_level: **high**（元合约 + 下游强制适配）
- 兼容：BDD-off MUST 不变；旧 tree.json 无 scenarios 字段仍可加载
