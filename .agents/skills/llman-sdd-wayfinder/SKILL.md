---
name: "llman-sdd-wayfinder"
description: "人类主动触发。将大型雾状工作（超出单个 agent session 容量）规划为 decision ticket map，逐个解决决策直到路径清晰。仅手动触发，agent 禁止自动启用。"
disable-model-invocation: true
metadata:
  version: "0.0.64"
  llman_sdd:
    bdd_mode: "on"
    skill_set: "optional"
---

# LLMAN SDD Wayfinder

一个模糊的大想法到来了——太大以至于单个 agent session 装不下，且包裹在雾里：从这里到**目的地**的路径还看不见。Wayfinding 是关于**找到那条路**，而非冲向目的地。

本 skill 把路径绘制为 llman SDD 的 **change 依赖图**（`llman sdd graph`），每个 ticket 解决一个**决策**而非交付一个切片，逐个解决直到路径清晰。

## Pipeline 位置

辅助工具，用于主 pipeline 之前的**大型工作预规划**。当 map 清晰后，合并到主流程 `llman-sdd-propose`。

> 📍 这是独立可选 skill；map 清晰后 → `llman-sdd-propose`（把决策收拢为可建计划）。

## 核心原则

- **Plan, don't do**：每个 ticket 解决一个决策，map 完成于「路径清晰，无决策遗留」。想直接动手的冲动通常是到了 map 边界、该交接的信号。
- **Refer by name**：在所有人类可读的叙述中，用 ticket 的标题名称指代，MUST NOT 用裸 id/编号。
- **单 session 单 ticket**：每个 session 只解决一个 ticket（research ticket 例外）。

## Map 结构

map 是一个 change（总纲 proposal），其子决策是它的 depends_on 子 change。用 `llman sdd graph <map-id> --scope active` 可视化 frontier。

map 的 `proposal.md` 结构：

```markdown
## Destination
<到达 map 终点是什么样——spec/决策/变更。一两行。>

## Notes
<领域；每 session 应查阅的 skill；该 effort 的常设偏好>

## Decisions so far
<!-- 索引：每个已关闭 ticket 一行，gist + 链接 -->

## Not yet specified
<!-- 雾：能预见但还无法精确成 ticket 的；随 frontier 推进毕业 -->

## Out of scope
<!-- 超出目的地的；关闭的 ticket，永不毕业 -->
```

## Ticket 类型

每个 ticket 是一个子 change，带 `wayfinder:<type>` 标注（在 proposal 的标题或 frontmatter）：

- **Research**（AFK）：读文档/API/本地资源以浮现决策所等的事实。委托 `llman-sdd-research` 后台解决。
- **Prototype**（HITL）：用廉价粗糙的可运行物（`/prototype` 思想：throwaway terminal app 或 UI 变体）提高讨论保真度。
- **Grilling**（HITL）：通过 `llman-sdd-explore` 的 grilling 分支逐问。**默认类型**。
- **Task**（HITL/AFK）：决策前必须发生的手动工作（注册服务、迁移数据以看清形状）。

## 雾的战争（Fog of war）

map **故意**不完整。能否成 ticket 的测试：**现在能否精确陈述该问题**（不是能否回答）。
- 能精确陈述 → ticket（即使被阻塞）。
- 还无法精确陈述 → **Not yet specified**（比 ticket 粗，一块雾可能毕业成多个 ticket 或零个）。

## 步骤

### 绘制 map（Chart）
1. **命名目的地**：用 `llman-sdd-explore` 的 grilling 分支钉死这趟 map 要找的路通往何方。
2. **广度优先扫 frontier**：再次 grilling，扇出而非深挖，浮现开放决策与现在可走的第一步。若**无雾浮现**——路径已清晰，整个旅程一个 session 能装下——不需要 map，停下问用户想怎么做。
3. **创建 map**（总纲 change）：`llman sdd change new <map-id>`，填 Destination/Notes，Decisions-so-far 空，雾写入 Not yet specified。
4. **创建现在能指定的 ticket**为子 change，然后 `llman sdd graph` 接线 blocking 边（second pass，需要 id 才能互引）。
5. 为每个 research ticket 启动 `llman-sdd-research` 后台 subagent。
6. 停——绘制是单 session 的活，不手解任何决策。

### 推进 map（Work through）
1. 加载 map（低分辨率视图）。
2. 选 ticket（用户指定或取 frontier 第一个），先 `change attach` claim 它。
3. 解决它——按需 zoom（读相关 ticket 全文，调用 Notes 指定的 skill）。存疑时用 `llman-sdd-explore` 的 grilling。
4. 记录解决：把答案作为 resolution 写入该 ticket 的 proposal，关闭它，并在 map 的 Decisions-so-far 追加一行 gist + 指针。
5. 新增 ticket（create-then-wire）；毕业答案让雾变成可指定的 ticket，从 Not yet specified 清除。若答案揭示某 ticket 越过目的地，归入 Out of scope 而非在路径上解决。

## 输出
map change + 子 decision change 的依赖图（`llman sdd graph`）。路径清晰后建议进入 `llman-sdd-propose` 把决策收拢为可建计划。

行动前先阅读 `llmanspec/config.yaml`，并遵循其中的 `context` 与 `rules`（若有）。

## Context
- 执行前先确认当前 change/spec 状态。
- map 与 ticket 都用 llman 的 change 机制承载（非外部 issue tracker）。

## Goal
- 把模糊大工作收敛为路径清晰的 decision map，产出的决策可供 propose 收拢。

## Constraints
- 变更保持最小化且范围明确。
- 标识符或意图不明确时禁止猜测。
- 单 session 单 ticket（research 例外）。
- 领域语义以 `spec.toon` 为 SSOT。

## Workflow
- 以 `llman sdd graph` 与 change 依赖为 map 的事实来源。
- ticket 解决的决策回写各自 proposal + map 的 Decisions-so-far。

## Decision Policy
- 高影响歧义必须先澄清。
- 已知校验错误下禁止强行继续。

## Output Contract
- 汇总 map 结构与 frontier 状态。
- 给出 change 路径与 `llman sdd graph` 输出。

## Ethics Governance
- `ethics.risk_level`：按 `low|medium|high|critical` 标注风险等级。
- `ethics.prohibited_actions`：列出绝对禁止执行的动作。
- `ethics.required_evidence`：列出高影响输出前必须具备的证据。
- `ethics.refusal_contract`：定义何时拒答以及安全替代响应方式。
- `ethics.escalation_policy`：定义何时必须升级为用户确认/人工复核。
