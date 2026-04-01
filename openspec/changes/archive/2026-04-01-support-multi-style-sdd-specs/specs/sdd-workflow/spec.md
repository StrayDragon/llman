# sdd-workflow Specification (Delta)

## MODIFIED Requirements

### Requirement: SDD 初始化脚手架
`llman sdd init [path]` 命令 MUST 在目标路径创建 `llmanspec/` 目录结构，包括 `llmanspec/AGENTS.md`、`llmanspec/project.md`、`llmanspec/config.yaml`、`llmanspec/specs/`、`llmanspec/changes/` 与 `llmanspec/changes/archive/`。命令 MUST 在 `llmanspec/config.yaml` 顶部写入 `yaml-language-server` schema 头注释，指向 `llmanspec-config` schema URL。命令 MUST 创建或刷新 repo 根目录下的 `AGENTS.md` 受管块以指向 `llmanspec/AGENTS.md`。当 `llmanspec/` 已存在时，命令 MUST 报错并且不修改任何文件。生成的 `llmanspec/AGENTS.md` MUST 包含 LLMANSPEC 受管提示块且包含完整 llman sdd 方法论说明。

初始化生成的 `llmanspec/config.yaml` MUST 包含：

- `locale`
- `version`
- 显式的 `spec_style: ison`

系统 MUST NOT 依赖“缺失 `spec_style` 时默认按 ISON 继续”的隐式行为。

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
- **AND** 配置包含 `spec_style: ison`

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

### Requirement: SDD 命令范围
`llman sdd` MUST 暴露以下命令集合：`init`、`update`、`update-skills`、`list`、`show`、`validate`、`archive`、`import`、`export`、`convert`。
其中：

- `import` 与 `export` MUST 作为 `llmanspec` 与外部规范目录互转的唯一入口。
- `convert` MUST 作为 `llmanspec` 内部 `ison / toon / yaml` 风格之间显式迁移的入口。

实现 MUST NOT 暴露 `migrate --from/--to` 兼容别名。
在 SDD 子命令组中 MUST 不提供 `change`、`view`、`completion`、`config` 等额外子命令。

#### Scenario: 帮助文本包含 import/export/convert
- **WHEN** 用户执行 `llman sdd --help`
- **THEN** 帮助文本包含 `import`、`export` 与 `convert`
- **AND** 帮助文本不包含 `migrate`

#### Scenario: style 参数强约束
- **WHEN** 用户执行 `llman sdd import` 或 `llman sdd export` 且缺少 `--style`
- **THEN** 命令返回非零并提示 `--style openspec` 为必填

#### Scenario: 旧命名不可用
- **WHEN** 用户执行 `llman sdd migrate --from openspec`
- **THEN** 命令返回未知子命令错误
