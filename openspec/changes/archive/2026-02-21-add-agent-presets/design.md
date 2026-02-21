## Context

`llman skills` 已经提供了稳定的能力：扫描 `<skills_root>` 找到 `SKILL.md` 技能包，并通过交互式 TUI 选择 target（claude/codex/agent + scope）后，用差异同步在目标目录中创建/移除 `<skill_id>` 的软链接。该机制非常适合把一组技能“注入”到 Codex / Claude Code 的技能目录中，但它缺少一个“可复用的组合单元”：

- 用户希望把“路由逻辑（system prompt）+ 一组技能清单”保存成一个 preset
- 未来希望支持 AI 生成 preset（模仿 skill-compose 的 agent-builder）
- 希望进一步生成可运行的框架代码（pydantic-ai / crewai）快速验证

由于 llman 目前定位为本地 CLI（非服务端运行时），我们不引入 Skill Compose 那种 `agent_id` 服务端选择机制；而是将 preset 设计为文件系统可见、可审阅、可复用的 artifacts，并继续复用 `llman skills` 的注入能力。

## Goals / Non-Goals

**Goals:**
- 新增 `llman agents` 命令，用于创建 agent preset 的本地工件（agent-skill + manifest）。
- agent id 与 skill id 统一：agent 本身就是一个 skill（`<skills_root>/<id>/SKILL.md`）。
- `llman skills` TUI 增加 Agents/Presets 区块：从 manifest 读取 includes 清单，一键勾选并交由既有 diff 同步注入。
- 新增 `llman agents gen-code`：基于 preset 生成单模块代码（pydantic-ai / crewai），用 `minijinja` 渲染模板并注入 prompt。
- 可选：`llman agents new --ai` 通过 `adk-rust` 实现本地 agent-builder，读取环境变量（`OPENAI_API_KEY` 等）生成 preset 内容。

**Non-Goals:**
- 不实现 Skill Compose 风格的服务端运行时（不提供 `agent_id` 执行入口）。
- 不在本变更中实现网络 skills 的 import/update/sources 管理链路（仅预留 manifest 元信息字段）。
- 不生成完整工程脚手架（`gen-code` 仅生成可运行的最小单模块，即单个 Python 文件）。

## Decisions

### Decision 1: Agent preset 的落盘形态 = agent-skill + agent manifest

为了在 Codex / Claude Code 的“技能目录”生态中自然工作，agent preset 由两部分组成：

1) **agent-skill（必需）**：一个普通技能目录，skill_id 即 agent id：
   - 路径：`<skills_root>/<id>/SKILL.md`
   - 内容：作为 system prompt 的载体（包含 routing/decision logic 与 `## Requirements` 段落）

2) **agent manifest（必需）**：一个机器可读的 preset 文件，用于 TUI 与 codegen：
   - 路径：`LLMAN_CONFIG_DIR/agents/<id>/agent.toml`
   - 字段（v1）：
     - `version = 1`
     - `id = "<id>"`
     - `description = "..."`
     - `includes = [...]`：依赖技能列表（不含自身 `<id>`）
     - `[[skills]]`（可选，替代/补充 includes）：每项包含 `id = "<skill_id>"`，可选 `path = "<path>"` 用于记录来源路径元信息；其它未知字段被忽略（前向兼容），为未来网络 skills 链路预留

> 约束：技能扫描仍以 `<skills_root>` 为唯一来源。manifest 目录不参与技能发现，仅用于 preset 展示与展开勾选。

### Decision 2: `llman skills` TUI 通过 manifest 实现 Presets/Agents 区块

TUI 在进入 skills 多选阶段时：

- 读取 `LLMAN_CONFIG_DIR/agents/*/agent.toml`
- 将其渲染为一个 **Agents/Presets** 区块条目（例如 `[agent] foo (3 skills)`）
- 选中该条目时，将 `<id>` + `includes` 中的 skill_ids 视为“期望选中集合”，并自动勾选对应 skills
- 用户仍可手动勾选/取消 individual skills，最终提交给既有 `apply_target_diff` 差异同步逻辑

该设计确保：
- 不新增 presets 专用 CLI flags（保持 `llman skills` 接口简洁）
- 注入语义仍由现有 target diff 统一实现（减少新写副作用逻辑）

### Decision 3: agent-skill 的 UI 标识与类型区分

agent 本身也是 skill，需要在列表中清晰区分：

- 若 `skill_id` 同时出现在 `agents/*/agent.toml` 的 `id` 字段集合中，则该 skill 在列表中显示为 `[agent] <id>`
- 其它 skills 保持原 label（`<skill_id> (<dir_name>)` 等）

### Decision 4: `agents gen-code` 用 minijinja 渲染“最小可运行模块”

`llman agents gen-code <id> --framework pydantic-ai|crewai --out <dir>`：
- 输入：agent id（读取 agent-skill 与 manifest）
- 输出：一个最小单模块代码（Python），默认从环境变量读取 OpenAI-compatible 配置
- 模板来源：新建 `templates/agents/<framework>/**`（或 include_str! 内嵌），用 `minijinja` 填充：
  - `SYSTEM_PROMPT`：来自 agent-skill（优先使用 `SKILL.md` 中适合作为 system prompt 的内容）
  - `INCLUDED_SKILLS`：manifest includes 列表（用于注释/日志/二次拼接）

框架行为参考 `llmarimo` 中 `pydantic_ai` 与 `crewai` 的 openai-compat 配置模式，但不依赖 marimo。

### Decision 5: `agents new --ai` 使用 adk-rust，且必须为可选 feature

`adk-rust` workspace较大且引入多 provider 支持，会显著增加默认编译与依赖体积；因此：
- 以 Cargo feature 方式启用（例如 `agents-ai`）
- 未启用时，`--ai` 返回明确错误并提示如何启用
- builder 输出必须为严格 JSON（由 llman 写文件），避免 LLM 直接执行文件写入操作

## Risks / Trade-offs

- **TUI 复杂度增加**：Presets 与 individual skills 混合选择易引起困惑。
  - 缓解：清晰区块标题、`[agent]` 标识、以及“选中 preset 会自动勾选 includes”的提示文案。
- **adk-rust 依赖成本**：编译时间与二进制体积上涨。
  - 缓解：feature gate；CI 可增加一个可选 job 覆盖该 feature。
- **Prompt 注入过大**：把多个 skills 的全文注入到代码中会造成体积过大与运行成本上升。
  - 缓解：默认仅注入 agent-skill 作为 system prompt；对 included skills 仅注入简短摘要或仅记录 skill_ids。

## Migration Plan

- 新增目录/文件均在 `LLMAN_CONFIG_DIR` 下，不影响现有用户。
- `llman skills` 的非交互模式行为保持不变；交互模式新增 Presets 区块。
- 用户工作流：
  1) `llman agents new <id>`（或 `--ai`）
  2) 编辑 `<skills_root>/<id>/SKILL.md`（手写模式）
  3) `llman skills` → 选择 target → 选择 agent preset/skills → apply
  4) 可选：`llman agents gen-code <id> ...` 生成代码快速验证
