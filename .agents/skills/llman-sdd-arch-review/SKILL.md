---
name: "llman-sdd-arch-review"
description: "扫描 codebase 的 shallow module 产出 deepening 候选。当用户想做架构审查、寻找 deepening 机会、提到 shallow module、或想改善代码可测性与 AI 可导航性时使用。"
metadata:
  version: "0.0.64"
  llman_sdd:
    bdd_mode: "on"
    skill_set: "optional"
---

# LLMAN SDD Architecture Review

扫描 codebase 的架构摩擦，提出 **deepening opportunities**——把 shallow module 变深的重构。目标是可测性与 AI 可导航性。

## Pipeline 位置

辅助工具，不属于主实现 pipeline（explore→propose→apply→verify→archive）。任意阶段可用，常在 explore 阶段触发以发现改进候选。

> 📍 这是独立可选 skill，不替代任何 pipeline 阶段。

## 设计词汇（借自 codebase-design，r108 固化）

使用以下词汇，MUST NOT 替换为「component」「service」「API」「boundary」：

- **Module** — 有 interface 和 implementation 的任何东西（函数/类/包/跨层切片）。
- **Interface** — 调用者为正确使用所须知道的一切（类型签名 + 不变量 + 顺序约束 + 错误模式 + 性能特征）。
- **Depth** — interface 处的杠杆：调用者每单位 interface 能驱动的行为量。**深** = 大行为 behind 小 interface；**浅** = interface 几乎和 implementation 一样复杂。
- **Seam** — 无需就地编辑即可改变行为的位置（interface 的栖身处）。
- **Adapter** — 在 seam 处满足 interface 的具体物。
- **Leverage** — 调用者从 depth 获得：更多能力 per 单位 interface。
- **Locality** — 维护者从 depth 获得：变更/bug/知识/验证集中一处。

## 步骤

### 1. 探索（先 scope，YAGNI）
- 若用户指定了方向（模块/子系统/痛点），直接采信，跳过推断。
- 否则回看 `git log --oneline` 找热点（反复出现的文件/区域）。
- 优先读 live `spec.toon`（BDD-on，领域语义 SSOT）与 `design.md`（已有 ADR），MUST NOT 另建 `CONTEXT.md`。
- 用 Agent 工具（subagent_type=Explore）走查 codebase，记录摩擦点：
  - 理解一个概念是否要在多个小 module 间跳转？
  - 哪里 module **浅**（interface 几乎和 implementation 一样复杂）？
  - 哪里纯函数仅为可测性抽取，但真实 bug 藏在调用方式里（无 locality）？
  - 哪些部分未测或难以通过当前 interface 测试？

### 2. 提出候选
对每个候选，给出：
- **Files** — 涉及哪些文件/module。
- **Problem** — 当前架构为何造成摩擦（用 depth/leverage/locality 词汇）。
- **Solution** — 会改变什么的平实描述。
- **Benefits** — locality 与 leverage 的改善，测试如何变好。
- **Recommendation strength** — `Strong` / `Worth exploring` / `Speculative`。

**deletion test**：对任何疑似浅的 module，想象删除它——复杂度是消失（pass-through）还是在 N 个调用点重现（它有价值）？「重现」才是你要的信号。

**ADR 冲突**：若候选与既有 `design.md` 决策矛盾，仅在摩擦真实到值得重开时才浮现，并在候选中标注（「与 design.md 的 X 决策冲突——但因…值得重开」）。

### 3. grilling（用户选定候选后）
用户从候选中选一个后，运行 `llman-sdd-explore` 的 **grilling 深对齐分支**（触发词「grill」）走决策树——约束、依赖、加深后的 module 形状、seam 后放什么、哪些测试存活。

- 命名一个加深后的 module 用了 `spec.toon` 没有的概念？→ 更新 `spec.toon` requirement statement（r107，BDD-on 在 feature 分支编辑 live 文件）。
- 用户以 load-bearing 理由拒绝候选？→ 仅当「难逆转 + 无上下文会困惑 + 真实权衡」三者皆满足时，建议记入 `design.md`。

## 输出
候选清单（文本；可选 HTML 报告写 OS temp dir 不落 repo）+ 用户选定后的 grilling 决策记录（回写 proposal/spec.toon）。

行动前先阅读 `llmanspec/config.yaml`，并遵循其中的 `context` 与 `rules`（若有）。

## Context
- 执行前先确认当前 change/spec 状态。
- 领域语义以 `spec.toon` 为 SSOT，MUST NOT 另建 glossary。

## Goal
- 产出可执行的 deepening 候选，并经 grilling 收敛为一个可进入 propose 的改进方向。

## Constraints
- 变更保持最小化且范围明确。
- 标识符或意图不明确时禁止猜测。
- 候选用 codebase-design 词汇，MUST NOT 漂移为 component/service/API。

## Workflow
- 以 live `spec.toon` 为领域语义事实来源。
- 候选经 grilling 收敛后，建议进入 `llman-sdd-propose`。

## Decision Policy
- 高影响歧义必须先澄清。
- ADR 冲突仅在摩擦真实时才重开。

## Output Contract
- 汇总候选与 recommendation strength。
- 用户选定后给出 grilling 决策摘要与回写路径。

## Ethics Governance
- `ethics.risk_level`：按 `low|medium|high|critical` 标注风险等级。
- `ethics.prohibited_actions`：列出绝对禁止执行的动作。
- `ethics.required_evidence`：列出高影响输出前必须具备的证据。
- `ethics.refusal_contract`：定义何时拒答以及安全替代响应方式。
- `ethics.escalation_policy`：定义何时必须升级为用户确认/人工复核。
