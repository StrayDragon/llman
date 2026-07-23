---
name: "llman-sdd-research"
description: "以后台 agent 委托外部文献调研。当用户需要针对某个问题查阅官方文档/API/源码等一手资料、或想把阅读文献的活委托给后台 agent 时使用。"
metadata:
  version: "0.0.64"
  llman_sdd:
    bdd_mode: "on"
    skill_set: "optional"
---

# LLMAN SDD Research

启动一个**后台 agent** 做文献调研，这样你可以继续手头工作而它在读。

## Pipeline 位置

辅助工具，任意阶段可用。常见于 explore/wayfinder 阶段，为决策提供事实输入。产出回写 change 的 proposal「Further Notes」段，供后续阶段引用。

> 📍 这是独立可选 skill；调研产出供主流程的 explore/propose 消费。

## 职责

后台 agent 的工作：

1. 针对**一手资料**调研问题——官方文档、源码、spec、第一方 API——而非对它们的二手转述。把每个论断追溯到拥有它的源头。
2. 把发现写入单个 Markdown 文件，为每个论断标注来源引用。
3. 存放位置：优先匹配仓库既有约定（如 `docs/research/`）；若无，放在 `llmanspec/changes/<current-change>/research/<topic>.md` 并说明位置。

## 步骤

1. 明确要调研的问题（与用户确认；模糊时收窄到一个能被证实/证伪的问题）。
2. 用 Agent 工具 `subagent_type=general-purpose` + `run_in_background: true` 启动后台调研，prompt 含：
   - 问题陈述。
   - 要求只引一手资料，每条论断标注来源 URL/路径。
   - 输出文件路径（`llmanspec/changes/<id>/research/<topic>.md`）。
   - 字数上限（建议：聚焦事实，散文式叙述 < 1500 词）。
3. 后台运行期间继续主流程工作；完成后收到通知。
4. 读取产出，把关键结论摘要回写到当前 change 的 `proposal.md`「Further Notes」段（附文件指针）。
5. 若调研揭示需要决策，建议进入 `llman-sdd-explore` 的逐问深挖分支。

## 与 wayfinder 的协作

`llman-sdd-wayfinder` 的 research ticket 委托本 skill 后台解决；解决后回写 ticket proposal 并在 map 的 Decisions-so-far 记一行 gist。

行动前先阅读 `llmanspec/config.yaml`，并遵循其中的 `context` 与 `rules`（若有）。

## Context
- 执行前先确认当前 change/spec 状态。
- 调研是事实收集，MUST NOT 替代决策（决策仍交用户）。

## Goal
- 产出 cited markdown 事实文件，并回写摘要到 change 工件。

## Constraints
- 变更保持最小化且范围明确。
- 标识符或意图不明确时禁止猜测。
- 只引一手资料；二手转述仅作线索，用来追溯源头。

## Workflow
- 后台 agent 调研 → cited markdown → 回写 proposal Further Notes。

## Decision Policy
- 高影响歧义必须先澄清。
- 调研不替代决策。

## Output Contract
- 汇总调研问题、产出文件路径、关键结论摘要。

## Ethics Governance
- `ethics.risk_level`：按 `low|medium|high|critical` 标注风险等级。
- `ethics.prohibited_actions`：列出绝对禁止执行的动作。
- `ethics.required_evidence`：列出高影响输出前必须具备的证据。
- `ethics.refusal_contract`：定义何时拒答以及安全替代响应方式。
- `ethics.escalation_policy`：定义何时必须升级为用户确认/人工复核。
