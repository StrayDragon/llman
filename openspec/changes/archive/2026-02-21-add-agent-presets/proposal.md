## Why

当前 llman 已具备较成熟的 skills 扫描 + TUI 选择 + 目标路径 symlink 注入能力（用于 Codex / Claude Code 等），但缺少一个“可复用的 agent preset”抽象：用户很难把“一组技能 + 路由/约束 prompt”作为一个可版本化、可复用的单元保存下来，并在 `llman skills` 中一键选择/应用。

与此同时，Skill Compose 的核心体验（通过交互/LLM 生成 agent preset，然后一键使用）证明了“preset 作为组合单元”非常有效。我们希望在 Rust/CLI 侧复刻这种能力，但不引入服务端运行时：让 llman 继续专注于本地文件系统注入与开发者工作流。

## What Changes

- 新增 `llman agents` 命令族：
  - `llman agents new <id>`：创建一个可复用的 agent preset 脚手架，默认走“用户完全手写”路线：
    - 在 `<skills_root>/<id>/SKILL.md` 生成一个 **agent-skill**（skill_id 就是 `<id>`），用于承载系统提示词（routing / decision logic / requirements）。
    - 在 `LLMAN_CONFIG_DIR/agents/<id>/agent.toml` 生成一个 **agent manifest**，记录该 agent 需要包含的 skills 列表与来源/路径等元信息。
  - `llman agents new <id> --ai`（可选能力）：集成 `adk-rust` 实现一个本地 agent-builder，基于用户需求 + `llman` skills 清单生成：
    - agent-skill 的 `SKILL.md`（含路由逻辑与 Requirements 段落，风格参考 skill-compose）
    - agent manifest（包含 `includes` 与元信息）
    - 运行时直接读取 `OPENAI_API_KEY` 与 `OPENAI_MODEL`（可选 `OPENAI_BASE_URL`）环境变量，不依赖 codex 等 coding CLI。
  - `llman agents gen-code <id> --framework pydantic-ai|crewai --out <dir>`：生成用于快速验证的最小代码模块（仅生成单模块 `agent.py`），使用 `minijinja` 进行模板渲染，并将 agent-skill 注入到生成的代码中。模板与约定参考 `/home/l8ng/Projects/__straydragon__/llmarimo` 中的 `pydantic_ai` 与 `crewai` notebooks 的 openai-compat 配置模式。

- 扩展 `llman skills` 交互式 TUI（不新增 presets 专用 CLI 参数）：
  - 启动时读取 `LLMAN_CONFIG_DIR/agents/*/agent.toml`，将其作为 **Agents/Presets** 区块展示在 skills 多选列表中。
  - 在 UI 上明确标识 agent-skill（例如 `[agent] <id>`），与普通 skill 区分。
  - 选择某个 preset 时，自动勾选其 `includes` 中的 skills（用户仍可手动增删），最终仍由既有 target diff 同步逻辑负责注入/移除 symlink。

- 变更范围内暂不实现“网络 skills 收集链路”（import/update/sources 管理）。但 agent manifest 会保留必要的来源/路径元信息，为后续引入 `skills import/update` 预留空间。

## Capabilities

### New Capabilities

- `agents-management`: 创建并管理 agent preset（agent-skill + manifest），支持手写与可选的 AI builder。
- `agents-codegen`: 基于 agent preset 生成 pydantic-ai / crewai 的最小可运行代码模块，用于快速验证。

### Modified Capabilities

- `skills-management`: 在不改变“skills 扫描根目录”约束的前提下，为交互式 TUI 增加 Agents/Presets 区块，并为 agent-skill 提供明确的 UI 标识与一键勾选能力。

## Impact

- 新增模块：引入 `src/agents/**`（manifest 解析、模板生成、可选 builder 适配层）。
- 修改模块：`src/skills/cli/**`（TUI 列表展示与默认选择逻辑）与相关 i18n 文案键。
- 依赖变更：
  - 引入 `minijinja` 用于 `agents gen-code` 模板渲染。
  - `adk-rust` 以 Cargo feature 形式集成（例如 `--features agents-ai`），避免默认构建体积与编译时间显著增加。
- 数据落盘：新增 `LLMAN_CONFIG_DIR/agents/<id>/agent.toml` 与 `<skills_root>/<id>/SKILL.md`。
- 测试：新增单元/集成测试覆盖 manifest 解析、preset 勾选行为与 codegen 输出；测试必须使用 `TempDir`/`LLMAN_CONFIG_DIR`，不得触碰真实用户配置。
