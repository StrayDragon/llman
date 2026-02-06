## MODIFIED Requirements
### Requirement: Schema 校验命令
`llman self schema check` MUST 校验已生成的 schema 文件与样例配置；当 schema 无效或样例配置不符合 schema 时 MUST 返回非零退出码。样例配置来源按以下优先级选择：
1) 若对应配置文件存在，则使用真实文件内容（全局/项目/llmanspec）。
2) 若不存在，则使用默认配置实例作为样例。
若对应配置文件存在但无法读取或无法解析为有效 YAML，命令 MUST 直接失败（不得静默回退 defaults）。

#### Scenario: Schema 校验失败
- **WHEN** `llman self schema check` 发现 schema 无效或样例配置不匹配
- **THEN** 命令返回非零退出码并报告错误

#### Scenario: 使用真实全局配置作为样例
- **WHEN** `LLMAN_CONFIG_DIR/config.yaml` 存在且用户运行 `llman self schema check`
- **THEN** 命令使用该文件内容作为样例并对照 schema 校验

#### Scenario: 真实配置不可读/不可解析会失败
- **WHEN** `LLMAN_CONFIG_DIR/config.yaml` 存在但无法读取或无法解析为有效 YAML
- **THEN** `llman self schema check` 返回非零错误且不会回退到默认实例作为样例

## ADDED Requirements
### Requirement: project/llmanspec 配置路径必须通过 root discovery 解析
`llman self schema apply` MUST 通过发现合适的根目录（repo root 或最近的 config root）定位 project 与 llmanspec 的配置文件，而不是假设当前工作目录就是根目录。

#### Scenario: 在子目录运行 schema apply
- **WHEN** 用户在 repo 的嵌套子目录中运行 `llman self schema apply`
- **THEN** schema header 被应用到 `<repo_root>/.llman/config.yaml` 与 `<repo_root>/llmanspec/config.yaml`（当文件存在时），而不是写入子目录下的同名路径

### Requirement: schema header 应用必须最小侵入
应用 YAML LSP schema header 时，工具 MUST 以最小侵入方式规范化文件顶部的 schema header：确保文件顶部存在且仅存在一个有效的 schema header，并 MUST NOT 删除不相关的行或内容。

#### Scenario: 存在多条 header 行
- **WHEN** 某 YAML 文件顶部包含多条 `# yaml-language-server: $schema=...` 行
- **THEN** 工具将其重写为“顶部一条正确 header”，同时保留其余内容不变
