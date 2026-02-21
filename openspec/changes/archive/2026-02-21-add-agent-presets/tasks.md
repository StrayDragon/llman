## 1. Specs

- [x] 1.1 编写增量规范：`agents-management`、`agents-codegen`、`skills-management`（agent presets）。
- [x] 1.2 运行 `openspec validate add-agent-presets --strict --no-interactive` 并修复问题。

## 2. Manifest 与路径

- [x] 2.1 新增 agent manifest v1 数据结构与 TOML 解析/序列化（`agent.toml`），覆盖 `includes` 与 `[[skills]] id/path` 元信息字段。
- [x] 2.2 增加 manifest 读取失败/缺失字段的错误信息与跳过策略（用于 skills TUI 与 codegen）。

## 3. CLI: `llman agents new`

- [x] 3.1 新增 `llman agents` 命令族与 `new` 子命令，复用与 `llman skills` 一致的 `<skills_root>` 解析逻辑。
- [x] 3.2 实现 `agents new <id>`：在交互式终端提供 includes 的 TUI 多选（允许为空、取消为安全 no-op），并生成 `<skills_root>/<id>/SKILL.md`（含 frontmatter 与 `## Requirements` 占位）与 `LLMAN_CONFIG_DIR/agents/<id>/agent.toml`（`version=1`、`id`、`includes`）。
- [x] 3.3 实现已存在路径的安全失败策略（不产生部分写入），并增加 `--force` 覆盖重建行为。
- [x] 3.4 增加集成测试覆盖：创建、重复创建失败、交互取消不写入、`--force` 覆盖、`LLMAN_CONFIG_DIR` 隔离。

## 4. `llman skills` TUI: Agents/Presets

- [x] 4.1 在进入交互式 skills 多选前扫描 `LLMAN_CONFIG_DIR/agents/*/agent.toml` 并加载为 agent preset 节点。
- [x] 4.2 将 agent preset 以树形父节点展示（显示为 `[agent] foo (3 skills)`），选择时展开为 `foo` + `includes` 并去重。
- [x] 4.3 为 agent-skill 条目增加 `[agent]` 标识，同时保留 `skill_id (directory_name)` 信息。
- [x] 4.4 对 manifest 解析失败与 includes 缺失 skill_id 的情况输出明确警告，但不阻断会话。
- [x] 4.5 增加测试覆盖：preset 展开勾选、缺失技能跳过、`[agent]` label、搜索过滤不丢父节点。

## 5. CLI: `llman agents gen-code`

- [x] 5.1 引入 `minijinja` 并添加 `pydantic-ai` / `crewai` 的 `agent.py` 模板（单模块输出）。
- [x] 5.2 实现 `agents gen-code <id> --framework pydantic-ai|crewai --out <dir>`：读取 agent-skill 正文与 manifest，渲染并写入 `<dir>/agent.py`，并在注释块中输出 `includes` 与 `[[skills]]` 元信息。
- [x] 5.3 增加输出冲突策略（默认安全失败，`--force` 覆盖；交互环境提示确认覆盖）。
- [x] 5.4 增加测试覆盖：缺少 `SKILL.md` / manifest 的错误、渲染结果包含 system prompt 与 includes 注释块。

## 6. CLI: `agents new --ai`（feature gate）

- [x] 6.1 增加 `agents-ai` Cargo feature：未启用时 `--ai` 返回明确错误并提示启用方式。
- [x] 6.2 集成 `adk-rust` 作为本地 agent-builder：读取 `OPENAI_API_KEY` 与 `OPENAI_MODEL`（可选 `OPENAI_BASE_URL`）环境变量，生成 system prompt（含 `## Requirements`）与 `includes`，并由 llman 写入文件。
- [x] 6.3 增加最小测试覆盖（无真实网络调用）：feature gate 行为、builder 输出 JSON schema 校验。

## 7. Quality

- [x] 7.1 运行 `just check` 并修复新增问题。
