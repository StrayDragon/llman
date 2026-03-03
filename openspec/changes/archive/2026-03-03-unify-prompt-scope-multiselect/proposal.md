## Why

`llman prompts` 当前的 `--scope user|project|all` 在用户认知与实际行为上存在歧义（例如 `all` 被 project 分支短路后无法覆盖全局目标），并且 Codex 侧同时存在两套可用注入入口（`AGENTS*.md` 作为 project doc、`prompts/*.md` 作为 custom prompts），但 llman 目前缺少清晰的目标模型与交互选择。这导致注入链路不可预测，也增加了跨 app 使用成本。

## What Changes

- **BREAKING**：将 `--scope` 从单值枚举 `user|project|all` 升级为可多选集合语义，统一使用 `global|project`（支持重复参数与逗号列表），不再保留 `user/all`。
- Codex 注入目标细分为两类，并可在 `global|project` 两层分别选择：
  - `agents`：注入到 `AGENTS.md`（或 `AGENTS.override.md`），用于 Codex project doc 指令发现
  - `prompts`：导出到 `prompts/<name>.md`，用于 Codex custom prompts
- Codex `agents` 目标支持 `--override`：将输出从 `AGENTS.md` 切换为 `AGENTS.override.md`（global/project 对应路径）。
- Claude Code 采用同一套 `global|project` 多选 scope 规则：
  - `global` → `~/.claude/CLAUDE.md`
  - `project` → `<repo_root>/CLAUDE.md`
- Cursor 适配同一“scope 集合”处理模型（统一参数解析与交互多选流程），当前仅支持 `project` 目标；若显式传入不支持 scope，返回错误。
- 多目标执行改为“逐目标解析、逐目标写入”，禁止因某一 scope/目标失败而提前阻断其他目标的写入尝试。
- 非交互模式下，已存在非托管目标文件仍要求 `--force`；交互模式下对该类目标增加二次确认。

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `prompts-management`: 调整 scope 语义、Codex 目标模型（agents/prompts）与 override 行为、冲突处理策略、多目标执行行为，统一 codex/claude-code/cursor 的 scope 处理模型。

## Impact

- Specs：更新 `prompts-management` 中 Codex/Claude scope 与路径定义、scope 参数语义、project 解析规则、冲突确认流程。
- Code：主要影响 `src/cli.rs`（scope 参数解析）与 `src/prompt.rs`（目标解析/写入策略/交互确认）。
- Tests：更新 `src/prompt.rs` 相关单元测试（路径断言、scope 组合、二次确认、非短路行为、无 git root 行为）。
- Docs/i18n：更新 `locales/app.yml` 的 scope/target 文案和交互提示，补齐 Codex 两类目标的描述。
