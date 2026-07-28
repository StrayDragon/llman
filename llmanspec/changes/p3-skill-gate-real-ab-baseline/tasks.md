# Tasks: p3-skill-gate-real-ab-baseline

## 垂直切片 1：promptfooconfig per-provider prompt override

- [ ] 1.1 在 `sdd_skill_gate_v1/promptfooconfig.yaml` 给 baseline/candidate provider 各加 `prompt: file://prompts/agent_task_<variant>.md` override
  - 验证：YAML 合法；两 provider 各有 prompt override

## 垂直切片 2：runner 生成两个独立 prompt 文件

- [ ] 2.1 改 `compose_agent_task` 调用：生成 `agent_task_baseline.md`（baseline skill）+ `agent_task_candidate.md`（candidate skill）
  (blocked-by: 1.1)
  - 验证：`--baseline-skill <snap> --no-run` 生成两个内容不同的 prompt 文件
- [ ] 2.2 无 `--baseline-skill` 时两文件内容相同（退化，不报错）
  (blocked-by: 2.1)
  - 验证：`--no-run`（无 baseline）两文件 diff 为空

## 垂直切片 3：git 快照持久化基线目录

- [ ] 3.1 新建 `agentdev/promptfoo/baselines/.gitkeep` + `baselines/README.md`（说明快照产生方式）
  - 验证：目录 + README 存在
- [ ] 3.2 runner 的 `--baseline-skill` 文档更新：示例指向 `agentdev/promptfoo/baselines/`
  (blocked-by: 3.1)
  - 验证：`--help` 示例含 baselines/ 路径

## 垂直切片 4：spec 精确化（r118 A/B + r120 基线）

- [ ] 4.1 修 `sdd-ab-evaluation/spec.toon` 的 r118：A/B 措辞精确化为「per-provider prompt override」
  - 验证：spec validate 通过
- [ ] 4.2 新增 r120：持久化基线存储（baselines/ 目录 + git 快照方式）
  (blocked-by: 4.1)
  - 验证：spec 含 r120 + scenario

## 垂直切片 5：门禁验证

- [ ] 5.1 dry-run 真 A/B：`--baseline-skill <snap> --no-run` 生成两个不同 prompt + per-provider override 正确
  (blocked-by: 2.1, 1.1)
  - 验证：两 prompt 文件 skill 段不同
- [ ] 5.2 `just check` 全绿
  (blocked-by: 5.1)
