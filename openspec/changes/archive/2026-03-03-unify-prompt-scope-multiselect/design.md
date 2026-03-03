## Context

当前 `prompts` 子系统存在三个耦合问题：
- scope 表达是单值枚举（`user|project|all`），无法直接表达“选择集合”，并带来 `all` 语义歧义。
- Codex 侧同时存在两套注入入口（project doc 的 `AGENTS*.md` 与 custom prompts 的 `prompts/*.md`），但 llman 缺少清晰的目标模型与交互选择。
- 目标解析流程以 project 分支为入口前置校验，导致组合 scope 可能被提前短路。

本变更聚焦 `llman prompts gen` 的 scope/target 语义与目标映射，不引入新框架，也不扩大到 prompts 以外的命令域。

## Goals / Non-Goals

**Goals:**
- 统一 scope 模型为“可多选集合”，对外语义仅保留 `global` 与 `project`。
- 为 Codex 明确区分两类注入目标：
  - `agents`：写入/注入 `AGENTS*.md`（project doc 指令链）
  - `prompts`：写入 `prompts/<name>.md`（custom prompts）
- 支持 Codex `agents` 的 `--override`：写入 `AGENTS.override.md`。
- 让 codex/claude-code/cursor 共用一套 scope 解析与验证流程（每个 app 自声明可用 scope 集合）。
- 消除组合 scope 的短路问题，保证逐目标尝试。

**Non-Goals:**
- 不保留 `user/all` 兼容别名（本次直接升级，非兼容过渡）。
- 不实现 Cursor 的用户级 rules 写入（当前仅 project 目标）。
- 不在本变更中重做模板存储结构（`LLMAN_CONFIG_DIR/prompt/<app>/` 保持不变）。

## Decisions

### 1) Scope 数据模型从枚举改为集合
- 将 `--scope` 从单值枚举改为可重复参数（`Vec`）+ 逗号拆分。
- scope 关键字固定为 `global|project`。
- 每个 app 定义 `supported_scopes(app)`：
  - codex: `{global, project}`
  - claude-code: `{global, project}`
  - cursor: `{project}`
- 用户传入 `user|all` 或其他非法值时直接报错；不提供迁移提示。

### 2) Codex 目标模型：agents vs prompts（均支持 global/project）
Codex 的两类入口语义不同：
- `prompts/*.md` 以“文件名 = prompt 名”使用
- `AGENTS*.md` 以“固定文件名 + 目录链路合并”使用

llman 必须显式建模，避免把“文件名 = 模板名”的模型硬套到 `AGENTS*.md`。

- `target=agents`：把一个或多个模板注入到 `AGENTS*.md`（托管块聚合，模板名仅作为区段标题）
  - global: `$CODEX_HOME/AGENTS.md`（默认 `~/.codex/AGENTS.md`）
  - project: `<repo_root>/AGENTS.md`
  - `--override` 将输出文件替换为 `AGENTS.override.md`（对应 global/project）
- `target=prompts`：把模板按“文件名 = 模板名”写到 prompts 目录
  - global: `$CODEX_HOME/prompts/<name>.md`（默认 `~/.codex/prompts/<name>.md`）
  - project: `<repo_root>/.codex/prompts/<name>.md`

默认行为（无显式 target）对 codex 选择 `prompts`，以保持最常见工作流（project scope → `.codex/prompts/`）稳定。

### 3) 逐目标执行 + 统一失败语义
- scope/target 解析后，按目标列表逐个解析与写入。
- project 解析失败不应阻止 global 目标的尝试。
- 只要任一目标失败，命令整体返回非 0；已成功目标不回滚。

### 4) 冲突处理策略（含交互二次确认）
- 非交互模式：目标文件存在且为非托管内容时，必须 `--force` 才允许修改。
- 交互模式：对上述场景执行二次确认（第一次确认继续，第二次确认风险）后才写入。

### 5) Cursor 的适配策略
- Cursor 也接入同一 scope 集合解析器与交互多选流程。
- 当前仅暴露 `project` 可选项；传入 `global` 时报错。
- 保持写入路径与扩展名不变：`<repo_root>/.cursor/rules/*.mdc`。

## Risks / Trade-offs

- [破坏性 CLI 变更] 旧脚本使用 `--scope user|all` 将失效
  → 缓解：在变更说明中明确 BREAKING，命令侧仅返回错误。

- [模型更复杂] Codex 多了 `target` 与 `override` 维度
  → 缓解：交互模式分步选择（scope → target → AGENTS/override），并在帮助文本中明确默认值。

- [部分成功语义] 多目标场景下可能出现“部分成功 + 非 0 退出”
  → 缓解：输出包含每个目标的成功/失败摘要。

## Migration Plan

1. 更新 spec（prompts-management）后先落地代码改造。
2. 更新 `--scope` 解析、交互提示与冲突确认流程，加入 codex 的 `target` 选择与 `--override` 目标映射。
3. 将 codex `agents` 注入实现为 managed block 聚合写入 `AGENTS*.md`。
4. 运行相关测试与 `openspec validate --strict`。
5. 在发布说明中标注 BREAKING：`user/all` 移除、codex scope/target 语义升级。

回滚策略：如需回滚，整体回退到本变更前提交，不保留半状态双轨逻辑。

## Open Questions

- None.
