# config-schemas Specification

## Purpose
TBD - created by archiving change add-config-schemas. Update Purpose after archive.
## Requirements
### Requirement: 生成 llman 配置 JSON schema
系统 MUST 生成配置的 JSON schema，并写入以下路径：
- `artifacts/schema/configs/en/llman-config.schema.json`（全局配置）
- `artifacts/schema/configs/en/llman-project-config.schema.json`（项目配置）
- `artifacts/schema/configs/en/llmanspec-config.schema.json`（llmanspec/config.yaml）

生成的 schema MUST 提供顶层与主要字段的 `title`/`description`，内容使用英文并与 CLI/i18n 术语保持一致。

#### Scenario: 生成 schema 文件
- **WHEN** 用户运行 `llman self schema generate`
- **THEN** 上述三个 schema 文件被写入（或刷新）到指定路径

#### Scenario: 生成 llmanspec schema
- **WHEN** 用户运行 `llman self schema generate`
- **THEN** `llmanspec-config.schema.json` 被写入（或刷新）到指定路径

### Requirement: YAML LSP schema 头注释
系统 MUST 支持以 `# yaml-language-server: $schema=...` 形式为 YAML 配置写入 schema 头注释，并确保该注释位于文件顶部（在 `---` 之前）。

`llman self schema apply` MUST：
- 为全局配置写入 `llman-config` schema URL
- 为项目配置写入 `llman-project-config` schema URL
- 为 llmanspec 配置写入 `llmanspec-config` schema URL
- 仅修改/修复头注释，不改写其他内容

#### Scenario: 头注释缺失
- **WHEN** 用户运行 `llman self schema apply` 且配置文件缺少 schema 头注释
- **THEN** 命令写入对应的 `yaml-language-server` 头注释

#### Scenario: 头注释不匹配
- **WHEN** 用户运行 `llman self schema apply` 且 schema URL 与目标不一致
- **THEN** 命令将其修复为正确的 schema URL

### Requirement: 项目配置 schema 为全局子集
项目配置 schema MUST 作为全局配置 schema 的子集，并排除仅允许在全局配置中定义的字段（例如 `skills.dir`）。

#### Scenario: 项目配置不包含全局专用字段
- **WHEN** `llman-project-config.schema.json` 被生成
- **THEN** schema 不包含 `skills.dir`

### Requirement: 首次运行生成全局样例配置
CLI 启动时若全局配置文件不存在，系统 MUST 生成样例配置并写入 schema 头注释。若文件已存在，系统 MUST NOT 修改现有内容。

#### Scenario: 首次运行生成样例配置
- **WHEN** CLI 启动且 `LLMAN_CONFIG_DIR/config.yaml` 不存在
- **THEN** 自动生成样例配置并写入 schema 头注释

#### Scenario: 已存在配置不被覆盖
- **WHEN** CLI 启动且 `LLMAN_CONFIG_DIR/config.yaml` 已存在
- **THEN** 该文件保持不变

### Requirement: Schema 校验命令
`llman self schema check` MUST 校验已生成的 schema 文件与样例配置；当 schema 无效或样例配置不符合 schema 时 MUST 返回非零退出码。

#### Scenario: Schema 校验失败
- **WHEN** `llman self schema check` 发现 schema 无效或样例配置不匹配
- **THEN** 命令返回非零退出码并报告错误

### Requirement: 运行时配置 schema 校验
系统 MUST 在读取配置文件时根据对应的 JSON schema 进行校验（全局/项目/llmanspec）。当配置不符合 schema 时 MUST 返回非零退出码并报告本地化错误。

#### Scenario: 全局配置不符合 schema
- **WHEN** CLI 读取 `LLMAN_CONFIG_DIR/config.yaml` 且内容与 `llman-config.schema.json` 不一致
- **THEN** 命令返回非零退出码并报告错误

#### Scenario: llmanspec 配置不符合 schema
- **WHEN** `llmanspec/config.yaml` 与 `llmanspec-config.schema.json` 不一致
- **THEN** 命令返回非零退出码并报告错误

