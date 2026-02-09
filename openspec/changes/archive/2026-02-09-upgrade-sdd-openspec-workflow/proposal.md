## Why

llman sdd 当前的 SDD 工作流（`llmanspec/` + `llman sdd update-skills`）提供了基础 skills，但与 OpenSpec 近期主推的 OPSX（`/opsx:*`）工作流存在明显落差：

- 缺少 OPSX 风格的 **slash commands + skills** 组合（尤其是 `/opsx:new`、`/opsx:continue`、`/opsx:apply` 等动作式入口）。
- 现有方案尝试通过“prompt 注入/占位符”降低重复，但会引入额外模板语法与加载优先级，增加维护复杂度与排查成本。

本变更希望用更直接、更可维护的方式：**把 OpenSpec 的 OPSX 命令与 skills 作为上游来源（vendor by copy）融合进 llman sdd**，并且 **只保留新的 OPSX 命令集合**（不再引入旧式 legacy commands）。

## What Changes

### 1) 引入 OPSX 工作流的“动作式入口”（slash commands）

- 为 Claude Code 与 Codex 生成 **OPSX 命令绑定**（`/opsx:*`），安装到工具约定目录：
  - Claude Code：`.claude/commands/opsx/`
  - Codex：`.codex/prompts/`（仅项目级；避免写入用户 home / user-global）
- 仅生成 OPSX 命令集合：`explore`、`onboard`、`new`、`continue`、`ff`、`apply`、`verify`、`sync`、`archive`、`bulk-archive`
- 当检测到 legacy 命令绑定（如 `.claude/commands/openspec/`、`.codex/prompts/openspec-*.md`）时，`update-skills` 将提示用户二次确认并执行迁移（删除 legacy 并生成 OPSX commands）

### 2) 融合 OPSX skills（不引入新的模板注入语法）

- 在 `templates/sdd/<locale>/` **直接复制并调整** OpenSpec OPSX 相关模板（来自上游 OpenSpec repo），作为 llman 的内置模板来源。
- 保持实现简单：不新增 `{{prompt: ...}}` 之类的模板占位符；仅使用现有 `{{region: ...}}` 能力（如仍需要共享片段）。
- 扩展 `llman sdd update-skills`：生成/刷新完整工作流 skills（覆盖 opsx 所需动作），并与 slash commands 的动作集合一一对应。

## Out of Scope

- 本次不实现 OpenSpec OPSX 的 schema engine / `status` / `instructions` 等 CLI 能力（这些属于更大范围的“schema-driven 工作流”，应在单独变更中推进）。
- 本次不实现自动化 delta specs 合并器（`sync` 仍以可复现的人工作业协议为主）。
- 本次不修改 `llman skills`（skills-management 能力）。

## Capabilities

### New Capabilities
- `sdd-opsx-commands`: 生成 OPSX slash commands（按工具适配目录输出）
- `sdd-opsx-workflow-skills`: 提供 OPSX 对齐的完整工作流 skills（与命令动作集合对应）

### Modified Capabilities
- `sdd-workflow`: 扩展现有 sdd 工作流，提供 OPSX（动作式）入口并更新 skills 生成内容

## Impact

- 受影响的 specs: `sdd-workflow`
- 受影响的代码:
- `src/sdd/project/update_skills.rs`: 扩展生成逻辑（skills + opsx commands）
- `src/sdd/project/templates.rs`: 扩展模板枚举与加载（新增 opsx 模板来源）
- `templates/sdd/*/skills/`: 补齐 opsx 工作流所需 skills
- `templates/sdd/*/spec-driven/`（或等价目录）: 追加 opsx 命令模板（vendor 并调整）
- `templates/sdd/*/agents.md`: 更新说明，允许/引导使用 opsx commands
