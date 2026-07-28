# Design: p2-skill-gate-draft-propose

## 核心思路：按 skill 定制 sandbox 预置

不同 skill 的「正确执行」需要不同的起始状态和验收点。P1 的 apply skill 预置了一个带 tasks.md 的 change；draft/propose 需要不同预置：

| skill | 起始状态 | agent 应做的事 | hard gate 验收 |
|---|---|---|---|
| apply | 带 tasks.md 的 change shell | 推进 1 个 task + 勾选 | `validate --strict` 通过 |
| draft | 空项目（仅 init） | `change new --from "<desc>"` 记草案 | `changes/<id>/proposal.md` 存在 + `validate` 通过 |
| propose | 空 spec + 一个 capability 目录 | 走 propose：建 change + 编辑 live spec + `change start` | `validate --strict` 通过 + change 进入 designed/full |

## seed_for_skill 分派

把现有 `seed_change_shell` 重构为 `seed_for_skill <skill_id> <workspace_dir> <config_dir>`，内部按 skill_id 分派：
- `llman-sdd-apply` → 现有 `seed_change_shell` 逻辑（预置 add-sample + tasks.md）
- `llman-sdd-draft` → no-op（空项目即可，任务由 task_prompt 驱动）
- `llman-sdd-propose` → 预置一个空 capability spec skeleton（让 agent 有合法的 spec 可编辑）

## task_prompt 按 skill 选择

runner 从 tests.yaml 里根据 `--skill` 选对应的 task_prompt（而非固定取第一个）。tests.yaml 结构改为按 skill 命名 description：

```yaml
- description: skill_gate_apply_end_to_end
  vars: { skill: llman-sdd-apply, task_prompt: ... }
- description: skill_gate_draft_end_to_end
  vars: { skill: llman-sdd-draft, task_prompt: ... }
- description: skill_gate_propose_end_to_end
  vars: { skill: llman-sdd-propose, task_prompt: ... }
```

runner 用 python 选 `skill == $SKILL_ID` 的那个用例的 task_prompt。

## r118 措辞修正（verify WARNING-2）

r118 里 `` `llman sdd update-skills --skills-only` 产物 `` → `` `llman sdd init --update` 产物 ``。功能等价，只是对齐实际 CLI。

## 不做的事（P2 边界）

- 不做真 A/B（per-variant prompt）——那是 P3。
- 不做持久化基线存储——那是 P3。
- 不改 hard gate 机制（`validate --strict` 对所有 skill 通用）。
