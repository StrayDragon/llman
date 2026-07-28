---
depends_on: []
branch: sdd/add-skill-gate-eval-baseline
base_sha: ca4d48560baaf03b8f771d4810ef2c9c564e37aa
checkpointed: false
---

## Why

当前 skill 模板迭代「几乎完全靠感觉」——根因是评估基础设施存在**结构性缺口**：两套现有 suite 各缺一半且从未组合。

| suite | skill-in-prompt? | sandbox+硬门禁? | A/B 基线? | 覆盖的 skill |
|---|---|---|---|---|
| `sdd_apply_v1` | ✅ 渲染 SKILL.md 进 system prompt | ❌ 仅 chat 决策（无真跑代码） | ⚠️ 退化（baseline==candidate=="new"） | apply（仅对话） |
| `sdd_llmanspec_styles_v1/v2` | ❌ 裸任务（prompt 无 SKILL.md） | ✅ sandbox + `sdd_gate.py` 硬门禁 + 批次统计 | ❌ 无 | 无（测 spec 格式，非 skill） |

「组合」从未发生：没有一个 suite 同时 = 渲染候选 SKILL.md 进 prompt + sandbox 跑硬门禁。此外**无持久化基线**——改一版模板无法客观判断变好还是变坏；`aggregate_batch_results` 只对同一配置多次 run 做风格维度聚合，无跨版本对比。propose/draft/quick/verify/archive 在 `agentdev/promptfoo/**` 下零覆盖。

## What Changes

**P1（本 change 范围）：组合最小闭环，先覆盖 apply skill**

1. **新建 fixture `agentdev/promptfoo/sdd_skill_gate_v1/`**：结构对齐 `sdd_llmanspec_styles_v1`（promptfooconfig.yaml + tests.yaml + prompts/ + assertions/），但 agent prompt = 渲染后的 SKILL.md（来自 `run-sdd-prompts-eval.sh` 的 `render_skill_prompt` 机制）+ agentic 任务。
2. **新建 runner `agentdev/promptfoo/run-sdd-skill-gate-eval.sh`**：组合两套现有零件——
   - 从 `run-sdd-claude-style-eval.sh` 复用 sandbox 建站（init 临时 llmanspec 项目 + git baseline + 隔离 LLMAN_CONFIG_DIR + llman binary on PATH）。
   - 从 `run-sdd-prompts-eval.sh` 复用 SKILL.md 渲染（`llman sdd update-skills --skills-only` → strip frontmatter → 包进 agent prompt）。
   - 接上 `sdd_gate.py` 式硬门禁（`llman sdd validate --all --strict`）。
3. **评分维度（MVP）**：硬门禁通过率（pass/fail）+ 成本（token/turns，来自 promptfoo 原生指标）。暂不含 LLM-rubric 软分（P3 再加）。
4. **基线锚点（MVP）**：`--baseline-skill <name>` 与 `--candidate-skill <path>` 对比——支持把上一版模板快照作为 baseline、当前工作区模板作为 candidate，A/B 对比。先不做 golden 参考（P3）。
5. **P1 测试用例**：apply skill 的 agentic 任务（agent 拿到渲染后的 apply SKILL.md + 一个带 tasks.md 的 change shell，需正确推进实现并通过 validate）。

## Capabilities

- `sdd-ab-evaluation`（扩 r118：skill-in-prompt × sandbox 硬门禁组合 + 跨版本 A/B 基线）

## Impact

- **新增**：`agentdev/promptfoo/sdd_skill_gate_v1/`（fixture）+ `agentdev/promptfoo/run-sdd-skill-gate-eval.sh`（runner）+ `scripts/sdd-skill-gate-eval.sh`（兼容入口 wrapper，对齐现有两个 wrapper 模式）。
- **不改**：现有两套 suite 保持不变（向后兼容）。
- **依赖**：复用 `@anthropic-ai/claude-agent-sdk`（已装）、promptfoo、llman binary。
- **后续（非本 change）**：P2 覆盖 draft/propose skill（依赖线1 promote-draft-skill 落地后的 draft skill）；P3 持久化基线存储 + LLM-rubric + golden 参考。
