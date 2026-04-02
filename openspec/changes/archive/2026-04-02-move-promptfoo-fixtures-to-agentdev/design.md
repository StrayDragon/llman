## Context

当前 Promptfoo fixtures 位于 `artifacts/testing_config_home/promptfoo/`。这会把“测试配置 fixture（用于 `LLMAN_CONFIG_DIR`）”与“评测套件（promptfoo prompts/tests/config）”混在一起。

随着评测场景升级（Claude Code、多轮交互、multi-style SDD、docker runner 等），我们需要一个更语义化、可扩展的入口目录来承载评测资产，避免 `artifacts/` 变成“杂物箱”。

## Goals / Non-Goals

**Goals:**
- 将 Promptfoo 评测套件从 `artifacts/` 迁移到仓库顶层 `agentdev/`，并形成稳定目录契约。
- 更新现有评测脚本以使用新路径，确保评测仍然可以在隔离临时目录中运行（不触碰真实用户配置）。
- 为后续“Claude Code + promptfoo agentic eval”与 docker runner 打底目录结构。

**Non-Goals:**
- 不在本变更中引入新的评测套件（仅迁移与整理现有套件）。
- 不改变现有评测逻辑/评分标准（仅路径与组织调整）。

## Decisions

1) **新增 `agentdev/` 顶层目录**
- 选择在仓库顶层引入 `agentdev/`，使其成为 agent/prompt 相关资产的唯一归属地。
- 不把这些内容放入 `docs/`：因为它们需要被脚本直接消费、可执行、可复制到临时目录中运行。
 - 对于入口脚本：优先在 `agentdev/` 内集中维护；如需兼容或便于发现，可在 `scripts/` 里保留薄封装（仅转发到 `agentdev/`）。

2) **保持 fixtures 的“平铺式”命名**
- 将现有 `promptfoo/<fixture>` 目录直接迁移到 `agentdev/promptfoo/<fixture>`（例如 `agentdev/promptfoo/sdd_apply_v1`），避免引入额外层级导致的路径升级成本。

3) **不做旧路径兼容**
- 迁移后脚本与文档只指向新路径，避免双入口导致的漂移与维护成本。

## Risks / Trade-offs

- [风险] 旧脚本/文档引用路径失效 → [缓解] 在同一个变更里同步更新脚本与相关规范；提供清晰的失败提示与路径说明。
- [风险] `artifacts/testing_config_home` 的语义被误解为“全部测试资产” → [缓解] 在规范与 README 中强调：`artifacts/testing_config_home` 仅用于 `LLMAN_CONFIG_DIR` 的测试配置 fixture。
