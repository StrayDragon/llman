---
depends_on: []
branch: sdd/p2-skill-gate-draft-propose
base_sha: ca4d48560baaf03b8f771d4810ef2c9c564e37aa
checkpointed: true
checkpoint_sha: ca4d48560baaf03b8f771d4810ef2c9c564e37aa
---

## Why

`add-skill-gate-eval-baseline`（r118）落地了 skill-gate 评测基础设施，但 P1 只覆盖 apply skill。draft / propose skill 仍无 eval 覆盖——这意味着改这两类 skill 模板时仍「靠感觉」。同时 verify 发现 r118 措辞引用了不存在的 `sdd update-skills` 命令（实际是 `sdd init --update`），需顺手修正。

## What Changes

1. **runner 支持按 skill 定制 sandbox 预置**：`seed_change_shell` 泛化为 `seed_for_skill`，根据 `--skill` 参数选择预置场景：
   - `llman-sdd-apply`：预置带 tasks.md 的 change shell（现状）
   - `llman-sdd-draft`：空项目（无 change），任务是用 `change new --from` 记一个草案
   - `llman-sdd-propose`：预置一个 idea 描述，任务是走完整 propose（建 change + live spec + attach）
2. **tests.yaml 增加对应场景**：draft / propose 各一个 agentic 任务用例。
3. **r118 措辞修正**：`llman sdd update-skills --skills-only` → `llman sdd init --update`（verify WARNING-2）。
4. **扩展 r118 覆盖声明**：从「P1 仅 apply」推进到「P2 覆盖 apply/draft/propose」。

## Capabilities

- `sdd-ab-evaluation`（r118：措辞修正 + 覆盖范围扩展；新增 r119：按 skill 定制 sandbox 预置）

## Impact

- **runner**：`run-sdd-skill-gate-eval.sh` 的 `seed_change_shell` 重构为按 skill 分派。
- **fixture**：`sdd_skill_gate_v1/tests.yaml` 增加 draft/propose 任务。
- **spec**：`sdd-ab-evaluation/spec.toon` 修 r118 措辞 + 加 r119。
- **不改**：hard gate 机制、聚合逻辑、promptfooconfig 结构（向后兼容）。
