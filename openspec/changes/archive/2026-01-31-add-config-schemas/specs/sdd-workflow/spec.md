## MODIFIED Requirements
### Requirement: SDD 初始化脚手架
`llman sdd init [path]` 命令 MUST 在目标路径创建 `llmanspec/` 目录结构，包括 `llmanspec/AGENTS.md`、`llmanspec/project.md`、`llmanspec/specs/`、`llmanspec/changes/` 与 `llmanspec/changes/archive/`，以及 `llmanspec/templates/spec-driven/` 下的 `proposal.md`、`spec.md`、`design.md`、`tasks.md`。命令 MUST 生成 `llmanspec/config.yaml` 并写入 locale 配置。命令 MUST 在 `llmanspec/config.yaml` 顶部写入 `yaml-language-server` schema 头注释，指向 `llmanspec-config` schema URL。命令 MUST 创建或刷新 repo 根目录下的 `AGENTS.md` 受管块以指向 `llmanspec/AGENTS.md`。当 `llmanspec/` 已存在时，命令 MUST 报错并且不修改任何文件。生成的 `llmanspec/AGENTS.md` MUST 包含 LLMANSPEC 受管提示块且包含完整 llman sdd 方法论说明。

#### Scenario: 初始化新项目
- **WHEN** 用户在不存在 `llmanspec/` 的目录执行 `llman sdd init`
- **THEN** 必要的目录结构与模板文件被创建

#### Scenario: 初始化指定路径
- **WHEN** 用户执行 `llman sdd init <path>`
- **THEN** 在 `<path>` 下创建 `llmanspec/` 结构与模板文件

#### Scenario: 初始化时生成提示块
- **WHEN** `llman sdd init` 生成 `llmanspec/AGENTS.md`
- **THEN** 文件中包含 `<!-- LLMANSPEC:START -->` 与 `<!-- LLMANSPEC:END -->` 包裹的提示块

#### Scenario: 初始化时写入配置
- **WHEN** `llman sdd init --lang en` 运行
- **THEN** `llmanspec/config.yaml` 被写入且 locale 为 `en`

#### Scenario: 初始化时写入 schema 头注释
- **WHEN** `llman sdd init` 生成 `llmanspec/config.yaml`
- **THEN** 文件顶部包含 `yaml-language-server` schema 头注释

#### Scenario: 初始化时生成根 AGENTS
- **WHEN** `llman sdd init` 运行
- **THEN** repo 根目录 `AGENTS.md` 被创建或刷新受管块并指向 `llmanspec/AGENTS.md`

#### Scenario: 已存在 llmanspec 目录
- **WHEN** 用户在已有 `llmanspec/` 的目录执行 `llman sdd init`
- **THEN** 命令返回错误且不做任何更改

#### Scenario: openspec 共存
- **WHEN** `openspec/` 已存在但 `llmanspec/` 不存在
- **THEN** `llman sdd init` 仅创建 `llmanspec/` 且不修改 `openspec/`
