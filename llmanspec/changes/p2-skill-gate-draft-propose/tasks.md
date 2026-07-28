# Tasks: p2-skill-gate-draft-propose

## 垂直切片 1：tests.yaml 增加 draft/propose 场景

- [ ] 1.1 在 `agentdev/promptfoo/sdd_skill_gate_v1/tests.yaml` 增加 draft 和 propose 的 agentic 任务用例（各含 `skill` 字段 + 对应 `task_prompt`）
  - 验证：YAML 合法；3 个用例各有 `skill` 字段
- [ ] 1.2 给现有 apply 用例补 `skill: llman-sdd-apply` 字段（保持结构一致）
  (blocked-by: 1.1)
  - 验证：apply 用例含 skill 字段

## 垂直切片 2：runner 按_skill 选 task_prompt

- [ ] 2.1 在 `run-sdd-skill-gate-eval.sh` 的 task_prompt 提取逻辑里，按 `--skill` 参数从 tests.yaml 选对应用例（而非固定取第一个）
  (blocked-by: 1.1)
  - 验证：`--skill llman-sdd-draft --no-run` 生成的 prompt 含 draft 专属 task_prompt

## 垂直切片 3：runner seed_for_skill 分派

- [ ] 3.1 把 `seed_change_shell` 重构为 `seed_for_skill <skill_id> <workspace_dir> <config_dir>`，按 skill 分派预置逻辑
  (blocked-by: 2.1)
  - apply：现有逻辑（预置 add-sample + tasks.md）
  - draft：no-op（空项目）
  - propose：预置一个空 capability spec skeleton
  - 验证：`--skill llman-sdd-draft --no-run` 下 workspace 无预置 change；`--skill llman-sdd-propose --no-run` 下有 spec skeleton

## 垂直切片 4：spec 扩展（r118 修正 + r119 新增）

- [ ] 4.1 修 `sdd-ab-evaluation/spec.toon` 的 r118 措辞：`update-skills --skills-only` → `init --update`；覆盖声明从「P1 仅 apply」推进到「P2 覆盖 apply/draft/propose」
  - 验证：`llman sdd validate sdd-ab-evaluation --strict --no-interactive` 通过
- [ ] 4.2 新增 r119：按 skill 定制 sandbox 预置（seed_for_skill 分派）
  (blocked-by: 4.1)
  - 验证：spec 含 r119 + 对应 scenario

## 垂直切片 5：门禁验证

- [ ] 5.1 dry-run 三个 skill：`--no-run` 下 apply/draft/propose 各自生成正确的 sandbox + prompt
  (blocked-by: 3.1, 2.1)
  - 验证：3 次 dry-run 产物结构正确
- [ ] 5.2 `just check` 全绿
  (blocked-by: 5.1)
