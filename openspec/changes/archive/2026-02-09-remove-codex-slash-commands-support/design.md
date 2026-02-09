## Context

当前 `llman sdd update-skills` 会为 Claude Code 与 Codex 同时生成 OPSX 命令绑定，其中 Codex 绑定落在 `.codex/prompts/opsx-*.md`。这与 Codex 对 Custom Prompts（slash commands）已弃用的方向冲突，也会让 SDD 指引继续推荐过时入口。与此同时，llman 已具备完整的 workflow skills 输出能力，足以覆盖 Codex 的主路径。

该变更需在不影响 Claude Code OPSX 体验的前提下，移除 Codex 的 command/prompt 绑定输出，并让 CLI 参数语义与模板文案保持一致。

## Goals / Non-Goals

**Goals:**
- 使 `llman sdd update-skills` 对 Codex 仅生成 skills，不再生成 `.codex/prompts/opsx-*.md`。
- 保留 Claude Code 的 OPSX commands 生成能力（`.claude/commands/opsx/`）。
- 明确 `--commands-only` 与 Codex 的组合语义，避免“命令成功但无输出”的静默行为。
- 更新 SDD 文档模板与规范描述，去除对 Codex slash commands 的推荐。

**Non-Goals:**
- 不移除 `llman prompts --app codex` 的既有能力（这是独立 capability）。
- 不重构 SDD workflow skills 结构与命名。
- 不引入新的跨工具命令绑定机制（仅收敛现有行为）。

## Decisions

### 1) 命令绑定支持矩阵收敛
- OPSX commands 仅对 Claude Code 生效。
- Codex 不再作为 commands 绑定目标，`update-skills` 在命令阶段跳过 Codex。

### 2) 参数语义保持最小破坏
- 默认模式（skills + commands）下：
  - `--tool codex` 仍成功，并仅生成 Codex skills。
  - `--all` 仍成功，生成 Claude skills+commands 与 Codex skills。
- `--commands-only` 下：
  - 若选择集合中没有 Claude（例如仅 `--tool codex`），命令 MUST 返回明确错误并提示改用 `--skills-only` 或选择 Claude。
  - 若同时包含 Claude（如 `--all --commands-only`），仅生成 Claude commands。

### 3) 迁移策略保持非破坏
- 本次不自动删除既有 `.codex/prompts/opsx-*.md` 文件，避免误删用户手动改写内容。
- legacy 清理逻辑继续仅覆盖历史 `openspec-*` 绑定迁移路径。

### 4) 文档与模板同步
- `templates/sdd/*/agents.md` 与 `templates/sdd/*/skills/shared.md` 中的 OPSX 快速上手描述更新为：Codex 使用 skills，不提供 slash commands/prompts 绑定。
- `sdd-workflow` 规范中相关 requirement/scenario 调整为工具差异化绑定模型。

## Risks / Trade-offs

- [兼容性落差] 依赖 `.codex/prompts/opsx-*.md` 的旧用户会失去“自动刷新”能力 → 在 CLI 错误/提示和模板文案中明确“Codex 改用 skills”。
- [行为歧义] `--commands-only --all` 可能被误解为“两个工具都生成” → 在规范与实现中显式定义为“仅 Claude 生成 commands”。
- [测试回归] 现有单元测试覆盖 Codex prompts 输出 → 同步替换为“Codex 不生成 commands”的断言，确保回归可见。

## Migration Plan

1. 更新 `sdd-workflow` delta spec，先冻结期望行为。
2. 修改 `update-skills` 逻辑与参数校验，限制 commands 目标为 Claude。
3. 更新中英文模板文案，去除 Codex slash commands 路径说明。
4. 更新单元/集成测试，验证 Codex skills-only 与 commands-only 边界行为。
5. 运行 `just test`（或最小相关测试）确认行为一致。

## Open Questions

- 是否在后续版本增加可选清理开关（例如 `--clean-deprecated-codex-prompts`）来移除历史 `opsx-*.md` 文件？本次先不引入。
