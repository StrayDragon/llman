# agents-codegen Specification (Change: add-agent-presets)

## ADDED Requirements

### Requirement: `agents gen-code` 生成最小可运行模块
`llman agents gen-code <id> --framework pydantic-ai|crewai --out <dir>` MUST 基于 agent preset 生成用于快速验证的最小代码模块：

- 输入：
  - agent id：`<id>`
  - agent-skill：从 `<skills_root>/<id>/SKILL.md` 读取（作为 system prompt 来源）
  - agent manifest：从 `LLMAN_CONFIG_DIR/agents/<id>/agent.toml` 读取（作为 includes 与元信息来源）
- 输出：
  - MUST 在 `<dir>` 下生成单个 Python 模块文件 `agent.py`
  - MUST 将 system prompt 注入到生成代码中（从 `SKILL.md` 正文提取，忽略 YAML frontmatter）
  - MUST 在生成代码中包含一个注释块，列出 manifest 的 `includes`，并在存在 `[[skills]]` 元信息时同时输出 `id/path`
- 生成过程 MUST 使用 `minijinja` 模板渲染，而不是拼接大量字符串

#### Scenario: 生成 pydantic-ai 模块
- **WHEN** 用户运行 `llman agents gen-code foo --framework pydantic-ai --out /tmp/foo`
- **THEN** `/tmp/foo/agent.py` 被创建，且包含 foo 的 system prompt 与 includes 注释块

#### Scenario: 生成 crewai 模块
- **WHEN** 用户运行 `llman agents gen-code foo --framework crewai --out /tmp/foo`
- **THEN** `/tmp/foo/agent.py` 被创建，且包含 foo 的 system prompt 与 includes 注释块

### Requirement: 生成代码必须使用 OpenAI-compatible 环境变量
生成的 Python 模块 MUST 从以下环境变量读取 OpenAI-compatible 配置，以便在不修改代码的情况下快速运行：
- `OPENAI_API_KEY`：必需
- `OPENAI_MODEL`：必需
- `OPENAI_BASE_URL`：可选；未设置时 MUST 默认使用 `https://api.openai.com/v1`

#### Scenario: 未设置 API key 时给出明确报错
- **WHEN** 用户直接运行生成的 `agent.py` 且未设置 `OPENAI_API_KEY`
- **THEN** 程序以非零退出并输出明确错误提示

#### Scenario: 未设置 model 时给出明确报错
- **WHEN** 用户直接运行生成的 `agent.py` 且未设置 `OPENAI_MODEL`
- **THEN** 程序以非零退出并输出明确错误提示

### Requirement: 输入不存在时失败策略明确
当 agent preset 不存在或不完整时，命令 MUST 返回错误：
- 若缺少 `<skills_root>/<id>/SKILL.md`，命令 MUST 返回错误
- 若缺少 `LLMAN_CONFIG_DIR/agents/<id>/agent.toml`，命令 MUST 返回错误

#### Scenario: 缺少 manifest
- **WHEN** 用户运行 `llman agents gen-code foo --framework pydantic-ai --out /tmp/foo` 且 `LLMAN_CONFIG_DIR/agents/foo/agent.toml` 不存在
- **THEN** 命令返回错误并提示先运行 `llman agents new foo`
