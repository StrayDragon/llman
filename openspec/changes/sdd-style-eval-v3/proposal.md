## Why（为什么）

当前 v2 的 `sdd-claude-style-eval` 已经做到“格式敏感”（强制使用 `Read` 工具 + 文件级编辑 + 严格校验），但任务仍偏“最小化验证”，不足以：

- 逼近真实 SDD 工作流中的多轮上下文构建（`propose -> apply -> archive -> commit` 的循环）
- 让不同规范风格（ison/toon/yaml）在**理解成本 / 编辑成本 / 工作流偏离率**上产生更稳定、可解释的差异

因此需要一个更“真实小项目场景”的“仅规范（不写代码）”评测：在同一 workspace 中连续完成 3 次 change 迭代，并引入可选的 rubric 评分（不影响硬门禁），以便同时观测：
1) 硬通过率（`validate`/约束是否满足）
2) 成本指标（turn/token/cost）
3) 软质量评分（rubric 分数）

## What Changes（变更内容）

- 新增 v3 Promptfoo 评测套件（`agentdev/promptfoo/sdd_llmanspec_styles_v3/`）：
  - 场景：以 TODO 应用为背景的“仅规范”小项目
  - 在同一对话/同一 workspace 内完成 **3 个 changes 的迭代循环**（`propose -> apply -> archive -> commit`）
  - `apply` 阶段允许使用 `llman sdd spec add-*` / `llman sdd delta add-*` 等 CLI 写入，但仍强制至少一次对 `spec.md` 的**文件级编辑**（并要求先 `Read`），确保格式差异真正进入上下文
  - 继续使用 `llman sdd validate --all --strict --no-interactive` 作为最终硬门禁
- runner/聚合增强（在不破坏 v1/v2 的前提下）：
  - 支持 `--fixture v3`
  - 支持 `--judge claude` 的“仅打分输出”（score-only，不作为通过/失败门槛）
  - 聚合报告纳入分数分布（均值/中位数/p90）与现有 token/turn/cost 指标一起输出

## Capabilities（涉及的规范）

### New Capabilities

<!-- none -->

### Modified Capabilities

- `sdd-ab-evaluation`: 扩展评测能力，覆盖“真实工作流循环 + 可选 rubric 评分 + 格式敏感的文件编辑约束”。

## Impact（影响范围）

- `agentdev/promptfoo/**`: 新增 v3 评测套件、断言、聚合字段（rubric 分数）
- `scripts/**` / `justfile`: 暴露 v3 与 judge（score-only）相关参数
- 评测成本会增大（3 次 change 循环），但所有产物仍只落在临时目录 `.tmp/`，便于回归与追踪
