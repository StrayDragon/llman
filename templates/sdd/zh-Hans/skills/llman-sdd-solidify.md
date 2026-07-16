---
name: "llman-sdd-solidify"
description: "将变更的 delta scenarios 序列化为可执行的 .feature 文件（仅 BDD-on）。在 apply 之后、archive 之前运行。框架无关：按 scenario 的 feature 字段和自指黑名单过滤后写入 Gherkin。"
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD Solidify

使用此 skill 为某个 change 生成（重新生成）可执行的 `.feature` 文件，来源是其 delta `spec.toon` 中的 scenarios。仅 BDD-on 项目。

## Pipeline 位置

```mermaid
flowchart LR
    apply["llman-sdd-apply<br/>实施"] --> verify["llman-sdd-verify<br/>验证"]
    verify --> solidify
    solidify["★ llman-sdd-solidify ★<br/>固化（你现在在这里）"]
    solidify --> archive["llman-sdd-archive<br/>归档"]
    archive --> commit["git commit<br/>完成闭环"]

    style solidify fill:#fff3cd,stroke:#ffc107,stroke-width:3px
```

> 📍 你现在在 solidify 阶段：verify 通过之后、archive 之前。
> BDD-off 项目：此命令为 no-op（无内容可生成）。

## 硬约束

- **BDD 模式感知**——先检查 `llmanspec/config.yaml` 是否含 `bdd:` 段，再分支：
  - **BDD-on**（有 `bdd:` 段）：正常执行 solidify（见下方步骤）。
  - **BDD-off，且 `llmanspec/specs/` 下无任何 `.feature` 文件**：no-op。报告「无需固化（BDD 未启用）」。
  - **BDD-off，但存在 `.feature` 文件**：报告**残留警告**——列出每个文件并说明：「发现 N 个 `.feature` 文件，但 BDD 未启用（`config.yaml` 无 `bdd:` 段）。它们会被 `validate`/`index` 忽略。若要重新启用可执行性，请添加 `bdd:` 段（如 `bdd:\n  run_command: \"cargo test --features bdd\"`）。有意重新启用，还是不再需要则删除？」**禁止删除这些文件**——只展示，由用户决定。
- **框架无关**：solidify 不扫描 `tests/bdd_steps.rs` 或任何 BDD 框架的 step 绑定。scenario 是否在运行时「可执行」由 `bdd.run_command` 判定。
- **禁止手工编辑 `.feature`**：它们是生成产物。改 `spec.toon` 的 scenarios，再重新运行 solidify。
- **不要问「要不要继续」**：一路执行到底，除非遇到无法自动解决的错误。

## 步骤

### 1) 确认目标 change
- 确定 change id（来自用户输入或上下文）。
- 始终说明："固化的变更：<id>"。
- `spec.toon` 是 SSOT。`.feature` 文件是其 scenarios 的**可执行子集**，序列化为 Gherkin。
- 当 scenario 的 `when` 调用 `llman sdd validate|archive|solidify` 时为**自指递归**，会被跳过（否则 BDD runner 会递归 spawn）。

### 2)（可选）Dry-run 预览
- `llman sdd solidify <id> --dry-run` 预览哪些 scenario 写入、哪些跳过。
- 检查跳过原因：`feature=false` 与自指 scenario 的跳过是预期的。

### 3) 执行 solidify
- `llman sdd solidify <id>`
- 会为每个 capability 在 `llmanspec/specs/<capability>/<capability>.feature` 写入一个文件。

### 4) 报告
- 汇总：每个 capability 写入/跳过的 scenario 数量，及输出路径。
- 跳过的 scenario 列出原因。

> 💡 上一阶段 `llman-sdd-verify`（已通过）→ 本阶段生成 `.feature` → 下一步 `llman-sdd-archive`（归档）。

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}
