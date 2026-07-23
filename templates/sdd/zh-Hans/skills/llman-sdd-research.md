---
name: "llman-sdd-research"
description: "以后台 agent 委托外部文献调研。当用户需要针对某个问题查阅官方文档/API/源码等一手资料、或想把阅读文献的活委托给后台 agent 时使用。"
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
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

`llman-sdd-wayfinder` 的查资料 ticket 委托本 skill 后台解决；解决后回写 ticket proposal 并在 map 的 Decisions-so-far 记一行要点。

{{ unit("skills/structured-protocol") }}
