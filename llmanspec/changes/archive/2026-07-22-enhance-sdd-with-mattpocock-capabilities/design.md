# Design: Enhance SDD with mattpocock.capabilities

## 背景：能力分析结论

mattpocock.skills 围绕"修复 AI 编码四大失败模式"组织（对齐不足 / 术语冗余 / 代码不工作 / 泥球架构）。核心主流程：`grill-with-docs`（对齐）→ `to-spec`/`to-tickets`（规划）→ `implement`+`tdd`（实现）→ `code-review`（审查）。

llman SDD 相对优势：可执行规格（BDD-on）、CLI 强校验、Git-native、分级 verify。相对短板：对齐深度浅、无设计词汇、自修复 ad-hoc、单轴审查。

## 方案选择：B（能力内化）vs A/C

| 方案 | 描述 | 否决理由 |
|------|------|----------|
| A 薄包装 | 复制改名 mattpocock skill | 双词汇并存（CONTEXT.md vs spec.toon）、无 CLI 支撑、维护成本高 |
| C 双轨 | 新增 router 调用 mattpocock | 集成度低，需用户记"何时用哪个增强" |
| **B 内化** | 熔进现有 pipeline | **选定**：单 SSOT、CLI 可支撑、词汇统一、渐进式 |

## 核心设计原则

### 1. 单 SSOT（Single Source of Truth）

mattpocock 用 `CONTEXT.md`（glossary）+ ADR 承载领域语义。llman 已有 `spec.toon` 的 `purpose` + `requirements[statement]` 承载领域语义，且 `design.md` 已是 ADR 的轻量版。

**决策**：不引入 `CONTEXT.md`。grilling 中 sharpening 术语时，更新 `spec.toon` 的 requirement statement；架构决策记入 `design.md`。避免双权威冲突。

### 2. 渐进式（Progressive Enhancement）

每项增强是 pipeline 阶段的**可选分支**：
- explore 默认行为不变；仅当用户说"深挖"/"grill"时进入 grilling 子模式。
- apply 默认最小修复；仅当判定为 hard bug 时升级 diagnose 子流程。
- verify 默认单轴；仅当用户要求"双轴"或 Standards 违规疑似时展开第二轴。

**理由**：不弱化现有门禁闭环的确定性；用户显式触发才付增强成本。

### 3. 先纯 SKILL.md，后 CLI

本 change 全部以 prompt 驱动（修改/新增 SKILL.md）。高频操作（如双轴 verify、arch-review）在模式验证稳定后，后续 change 再固化成 `llman sdd` 子命令。

**理由**：避免过早工程化；prompt 先行可快速迭代措辞；符合 `writing-great-skills` 的"predictability over output"原则。

## 各项能力落点设计

### r100: explore grilling 深对齐（P1a）

作为 explore 的**分支**（branch，非新 skill）。触发条件：用户显式说"深挖"/"grill"/"彻底理清"/"逐个问"。

循环结构（来自 mattpocock `grilling`）：
```
while 决策树未走完:
  问一个问题（只一个，附推荐答案）
  if 该问题可通过读 spec.toon/代码/运行命令确认:
    自行查证，不问用户
  else:
    等用户回答
  记录决策 → proposal.md "Open Questions" 段（BDD-on 写 feature 分支）
```

**完成判据**：决策树每一分支均已解决（resolved 或显式 deferred）。

### r101: propose seam 确认 + 垂直切片（P0a）

**seam 定义**：llman 的 seam = `*.feature` 的 GWT 步骤所驱动的公共边界（CLI 子进程、public interface）。复用 mattpocock `to-spec`/`tdd` 的 seam 概念，但**不另发明**——seam 来自已有的 `.feature` harness。

tasks.md 增强格式：
```markdown
## Seams under test
- [x] seam: CLI `llman sdd validate` (confirmed with user)

## Tasks (tracer-bullet, dependency-ordered)
- [x] task-1: <vertical slice> [blocked-by: none]
- [ ] task-2: <vertical slice> [blocked-by: task-1]
```

**原则**（来自 `to-tickets`）：每个 task 是垂直切片（cuts schema→API→UI→tests）；wide refactor 例外走 expand-contract。

### r102: apply diagnose 紧反馈循环（P1b）

apply 自修复失败时升级路径（来自 mattpocock `diagnosing-bugs` Phase 1）：
```
Round N 失败 → 判定是否 hard bug:
  是 → diagnose 子流程:
        1. 建 red-capable 命令（紧、确定、快、agent-runnable）
        2. 最小化 repro
        3. 生成 3-5 排序假设
        4. 单变量验证
  否 → 现有最小修复逻辑
```

**硬约束**：无 red-capable 命令禁止进入假设阶段（防止"盯着代码猜"）。

### r103: verify 双轴审查（P0b）

两轴分离（来自 mattpocock `code-review`），报告互不污染：

- **Spec 轴**（现有强化）：实现是否满足 `spec.toon` MUST/SHALL + `.feature` GWT。缺失/部分/scope creep/错误实现。
- **Standards 轴**（新增）：代码是否符合 `AGENTS.md` coding style + Fowler 12 smell baseline（Mysterious Name / Duplicated Code / Feature Envy / Data Clumps / Primitive Obsession / Repeated Switches / Shotgun Surgery / Divergent Change / Speculative Generality / Message Chains / Middle Man / Refused Bequest）。

**权威优先级**：`AGENTS.md` 文档标准 > smell baseline（repo overrides）；tooling 已强制的跳过。

### r104-r106: 独立可选 skill（P2）

不进线性 pipeline，按需触发：

| skill | invocation | 来源 | 产出 |
|-------|-----------|------|------|
| arch-review | model-invoked | `improve-codebase-architecture` | shallow module 候选 + deepening 建议 |
| wayfinder | user-invoked | `wayfinder` | decision ticket map（结合 `change`+`graph`） |
| research | model-invoked | `research` | cited markdown 回写 proposal Further Notes |

**注意 r90/r95**：model-invoked skill 需纳入 `config.yaml` 的 `extra_skills` 才能被 `init --update` 管理；user-invoked（wayfinder）可不纳入（用户手动 `/skill` 触发）。

### r107: 领域语言治理（P3a）

grilling/explore 中遇到术语冲突时：
- **挑战**："你的 spec.toon 定义 'X' 为 A，但你刚说成 B——哪个对？"
- **回写**：解决后更新 `spec.toon` requirement statement（feature 分支）。
- **ADR 谨慎**：仅在"难逆转 + 无上下文会困惑 + 真实权衡"三者皆满足时建议记入 `design.md`。

### r108: AGENTS.md 路由（P3b）

在 AGENTS.md 的 SDD 段新增"可选增强能力"小节，索引 P0-P2 触发条件。不另建 router skill（llman 已有 pipeline mermaid 图 + AGENTS.md，无需 `ask-matt` 式 router）。

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| 增强措辞过长拖慢 skill 加载 | 遵循 `writing-great-skills` 的 progressive disclosure：主干简洁，细节外链 |
| 双轴 verify 并行 sub-agent 成本 | 本 change 先串行实现；并行优化留后续 |
| mattpocock 词汇（seam/depth）与 llman 词汇混用 | r108 在 AGENTS.md 固化定义；复用已有概念时不另发明 |
| 独立 skill 增加 context load | model-invoked 的 description 严格裁剪触发词（writing-great-skills 原则） |

## 已知限制：3 个新 skill 尚未被 `init --update` 托管

`config.yaml` 的 `extra_skills` 合法值由编译期常量 `OPTIONAL_SKILL_NAMES`（new-change/continue/ff/sync/validate）限定；`src/sdd/project/config.rs` 会拒绝未知条目。因此 arch-review/wayfinder/research **暂无法**通过 `extra_skills` 纳入 `init --update` 候选集。

当前状态：
- 3 个 skill 文件已放入 `.agents/skills/llman-sdd-{arch-review,wayfinder,research}/`，含正确的 `metadata.llman_sdd`（bdd_mode=on, skill_set=optional）→ **`validate` 通过**（r95 只校验 metadata 一致性，不校验候选集归属）。
- 若用户运行 `llman sdd init --update`，r90 会清理这 3 个目录（因不在 default + extra_skills 候选集）。

**后续 CLI change 的范围**（符合"先 prompt 后 CLI"原则）：把这 3 个 skill 加入 `OPTIONAL_SKILL_FILES` + `OPTIONAL_SKILL_NAMES`，提供 `templates/sdd/{en,zh-Hans}/skills/` 模板，并更新 schema 的 valid values 描述。届时才可纳入 `extra_skills`。在此之前，用户手动保留这 3 个目录即可使用。
