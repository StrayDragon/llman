---
change_id: fix-sdd-bdd-on-change-stage
title: BDD-on：determine_stage / show / skills 认 attach+工件，不再强求 change/specs
status: full
depends_on: []
author: agent
branch: fix/sdd-bdd-on-change-stage
base_sha: 97eca13f0f6a2c183d4d978b7a2bb56f6f4767a7
checkpointed: true
checkpoint_sha: 0ec4be1e9a9922da0b3ec45011dceb32ba2ca237
---

# fix-sdd-bdd-on-change-stage

## Why

1. Git-native Partitioned SSOT（BDD-on）把合约写在 live `llmanspec/specs/**`，**禁止** `changes/<id>/specs/` delta。
2. `determine_stage` 仍按 BDD-off 形状：无 `change/specs/` → 永远 `draft`，即使已有 `proposal`+`design`+`tasks` 且 `attach`（`branch`/`base_sha`）。
3. Consumer（xylitol c1290）闭环中：`show`/`validate` 打出 `stage=draft`、`readyToImplement=false`、`next: add specs/`——**误导 agent**；verify/apply skill 模板仍写「非 full 则 STOP」，与 r61（流水线须 Git-native 感知）冲突。
4. 0.0.64 的 `improve-partitioned-ssot-agent-friction` 已修 dual-write 消息与 `checkpoint --no-interactive`，**未**修 stage 推断。

## Purpose

1. **BDD-on**：`determine_stage`（及 `show`/`list`/`status` 同源）在 change **已 attach** 且具备 `proposal.md`+`design.md`+`tasks.md` 时 MUST 报 `full`（不要求 `changes/<id>/specs/`）。
2. **BDD-off**：保持现有「看 `change/specs/`」逻辑不变。
3. completeness INFO：已 attach 的 BDD-on 完整工件 MUST NOT 再提示 `next: add specs/`；未 attach 时可提示 `next: attach`。
4. Skills（apply/verify/continue）：BDD-on 下以「工件 + attach / stage=full」放行；MUST NOT 仅因「无 change delta」拒绝。
5. 可执行场景锁定上述行为（`sdd-workflow`）。

## What Changes

- `src/sdd/spec/validation.rs`：`determine_stage`（及必要时 `list_change_artifacts`）读 config `bdd:` + proposal frontmatter attach 字段
- `src/sdd/shared/show.rs` / list / status：与 stage 同源
- locales：stage hint 文案
- `templates/sdd/*/skills/llman-sdd-{apply,verify,continue}.md`：BDD-on 守卫
- live：`sdd-workflow` `r93` + feature；必要时调整 skills-contract 场景措辞

## Capabilities

- `sdd-workflow`（modify）

## Out of scope

- 恢复 change delta / solidify
- 改 Totals 行格式
- xylitol 侧实现（仅文档指向本 change）

## Ethics

- risk_level: low
- prohibited_actions: 在 BDD-on 下重新要求 feature_delta；静默把 live specs 复制进 change/specs 冒充 full
- required_evidence: 单测或 BDD：attach + proposal/design/tasks → stage=full；BDD-off 有 specs 路径回归
- escalation_policy: 若 attach 字段 schema 变更需文档化迁移

## Impact

- Agent 在 xylitol 等 consumer 上不再被假 draft 拦住 apply/verify
- `readyToImplement` 对已就绪的 BDD-on change 变为 true