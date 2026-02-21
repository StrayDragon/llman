# agents-management Specification (Change: add-agent-presets)

## ADDED Requirements

### Requirement: 新增 `agents` 命令族
CLI MUST 提供 `llman agents` 作为顶层命令族，用于管理可复用的 agent preset。

#### Scenario: help 中包含 agents
- **WHEN** 用户运行 `llman --help`
- **THEN** 帮助信息包含 `agents` 命令

### Requirement: `agents new` 生成 agent-skill 与 agent manifest
`llman agents new <id>` MUST 在本地创建一个 agent preset，由两部分组成：

1) **agent-skill**（普通 skill 目录，用于承载 system prompt）：
- 路径：`<skills_root>/<id>/SKILL.md`
- `SKILL.md` MUST 包含有效 YAML frontmatter，且 `name` MUST 为 `<id>`（使其 skill_id 稳定等于 `<id>`）
- `SKILL.md` MUST 为用户预留可编辑的 system prompt 正文，以及 `## Requirements` 段落

2) **agent manifest**（机器可读清单，用于 TUI 与 codegen）：
- 路径：`LLMAN_CONFIG_DIR/agents/<id>/agent.toml`
- MUST 写入 `version = 1` 与 `id = "<id>"`
- MUST 写入 `includes` TOML 数组（默认值为 `includes = []`；且 MUST NOT 包含自身 `<id>`）

命令 MUST 使用与 `llman skills` 相同的 `<skills_root>` 解析逻辑（CLI 覆盖/ENV/全局配置/默认值）。
当输出路径已存在时，命令 MUST 默认安全失败；当传入 `--force` 时，命令 MUST 覆盖重建该 agent preset。

#### Scenario: 创建新的 agent preset
- **WHEN** 用户运行 `llman agents new foo-agent`
- **THEN** `<skills_root>/foo-agent/SKILL.md` 与 `LLMAN_CONFIG_DIR/agents/foo-agent/agent.toml` 均被创建

#### Scenario: 已存在时安全失败
- **WHEN** `<skills_root>/<id>` 或 `LLMAN_CONFIG_DIR/agents/<id>` 已存在且用户再次运行 `llman agents new <id>` 且未传入 `--force`
- **THEN** 命令返回错误且 MUST NOT 产生部分写入

#### Scenario: --force 覆盖重建
- **WHEN** `<skills_root>/<id>` 或 `LLMAN_CONFIG_DIR/agents/<id>` 已存在且用户运行 `llman agents new <id> --force`
- **THEN** 命令覆盖重建该 agent preset，并确保最终输出符合本规范

### Requirement: `agents new` 支持交互式 includes 选择（TUI）
当 `llman agents new <id>` 运行在交互式终端时，命令 MUST 提供一个 TUI 多选步骤用于配置 `agent.toml` 的 `includes`：
- 选项列表 MUST 来源于 `<skills_root>` 发现到的 skills（与 `llman skills` 相同的扫描规则）
- 选项展示 MUST 为 `skill_id (directory_name)`
- 默认选择 MUST 为空（用户可选择 0 个技能）
- 结果 MUST 写入 `LLMAN_CONFIG_DIR/agents/<id>/agent.toml` 的 `includes` 数组
- 命令 MUST 确保 `includes` 不包含 `<id>`（即使该 skill 已存在于 `<skills_root>`）

#### Scenario: 选择 includes 写入 manifest
- **WHEN** 用户在 `agents new` 的 TUI 中选择 `ripgrep-helper` 与 `mermaid-expert`
- **THEN** `agent.toml` 的 `includes` 包含 `ripgrep-helper` 与 `mermaid-expert`

#### Scenario: 允许选择为空
- **WHEN** 用户在 `agents new` 的 TUI 中不选择任何技能并确认
- **THEN** `agent.toml` 的 `includes` 为空数组

#### Scenario: 取消 TUI 不产生变更
- **WHEN** 用户在 `agents new` 的 TUI 中取消退出
- **THEN** 命令整体 abort 且 MUST NOT 写入任何文件或目录

### Requirement: agent manifest v1 支持 skill 元信息
agent manifest（`agent.toml`）v1 MUST 支持记录包含的 skills 列表及其可选元信息：

- `includes = ["skill-a", "skill-b"]`：用于交互式 TUI 的“展开勾选”
- 可选 `[[skills]]` 数组：每项 MUST 至少包含 `id = "<skill_id>"`，并 MAY 包含 `path = "<path>"` 用于记录该 skill 的来源路径元信息

未知字段 MUST 被忽略（前向兼容）。

#### Scenario: includes 与 skills 元信息可同时存在
- **WHEN** `agent.toml` 同时包含 `includes = ["skill-a"]` 与 `[[skills]]`
- **THEN** llman 仍能读取并使用 `includes` 作为展开集合，且保留 `[[skills]]` 元信息

### Requirement: `agents new --ai` 必须为可选 feature
当用户运行 `llman agents new <id> --ai` 时，命令 MUST 满足以下约束：
- 若二进制未启用 `agents-ai` feature，命令 MUST 返回明确错误并提示需要重新编译并启用 `--features agents-ai`
- 若启用该 feature，命令 MUST 使用本地 LLM client（基于环境变量 `OPENAI_API_KEY` 与 `OPENAI_MODEL`，可选 `OPENAI_BASE_URL`）生成：
  - agent-skill 的正文（包含 routing/decision logic，并在末尾包含 `## Requirements`）
  - agent manifest 的 `description` 与 `includes`

#### Scenario: 未启用 feature 的错误提示
- **WHEN** 用户运行 `llman agents new foo --ai` 且二进制未启用 `agents-ai`
- **THEN** 命令返回错误并提示需要启用 `agents-ai`
