---
depends_on: []
---

## Why

verify 发现 `add-skill-gate-eval-baseline` 的 CRITICAL-1：r118 要求的「跨模板版本 A/B 并行评测」未真正生效——baseline 和 candidate 两个 provider 共享同一个 prompt（都用 candidate skill 文本），A/B 退化为重复测量。根因是 runner 只把 candidate skill 注入了共享的 `prompts/agent_task.md`，baseline 变量算了却从未传入。同时 P1/P2 的「持久化基线」仍是空白——每次对比都要手动准备快照，无法客观判断模板迭代是变好还是变坏。

## What Changes

1. **per-provider prompt override（修复 CRITICAL-1）**：在 promptfooconfig.yaml 给 baseline/candidate provider 各加 `prompt` override（promptfoo 原生支持），baseline 用 baseline-skill 快照渲染的 prompt、candidate 用当前工作区模板渲染的 prompt。一次 eval run 真正并行 A/B。
2. **runner 生成两个独立 prompt 文件**：`prompts/agent_task_baseline.md`（baseline skill 注入）+ `prompts/agent_task_candidate.md`（candidate skill 注入），分别被两个 provider 的 override 引用。
3. **git 快照持久化基线**：新建 `agentdev/promptfoo/baselines/` 目录存历史 SKILL.md 快照（由 `git show <ref>:templates/.../SKILL.md` 产生）。runner 的 `--baseline-skill` 默认指向该目录，支持版本化基线对比。
4. **spec 精确化**：r118 的 A/B 措辞从「P1 单 prompt 双 provider」推进到「P3 per-provider 真 A/B」。

## Capabilities

- `sdd-ab-evaluation`（r118：A/B 措辞精确化到 per-provider；新增 r120：持久化基线存储）

## Impact

- **promptfooconfig.yaml**：两 provider 各加 `prompt: file://prompts/agent_task_<variant>.md` override。
- **runner**：`compose_agent_task` 调两次（baseline + candidate），patch 逻辑更新。
- **新增**：`agentdev/promptfoo/baselines/.gitkeep` + baselines/README。
- **不改**：hard gate、聚合逻辑、seed_for_skill 分派（向后兼容）。
