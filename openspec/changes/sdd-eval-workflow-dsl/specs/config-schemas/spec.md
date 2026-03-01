## MODIFIED Requirements

### Requirement: 生成 llman 配置 JSON schema
系统 MUST 生成配置的 JSON schema，并写入以下路径：
- `artifacts/schema/configs/en/llman-config.schema.json`（全局配置）
- `artifacts/schema/configs/en/llman-project-config.schema.json`（项目配置）
- `artifacts/schema/configs/en/llmanspec-config.schema.json`（llmanspec/config.yaml）
- `artifacts/schema/playbooks/en/llman-sdd-eval.schema.json`（`llman x sdd-eval` playbook）

`llman x sdd-eval` playbook schema MUST be publishable at the URL:
- `https://raw.githubusercontent.com/StrayDragon/llman/main/artifacts/schema/playbooks/en/llman-sdd-eval.schema.json`

生成的 schema MUST 提供顶层与主要字段的 `title`/`description`，内容使用英文并与 CLI/i18n 术语保持一致。

#### Scenario: 生成 schema 文件
- **WHEN** 用户运行 `llman self schema generate`
- **THEN** 上述 schema 文件被写入（或刷新）到指定路径

#### Scenario: 生成 sdd-eval playbook schema
- **WHEN** 用户运行 `llman self schema generate`
- **THEN** `llman-sdd-eval.schema.json` 被写入（或刷新）到指定路径

### Requirement: Schema 校验命令
`llman self schema check` MUST 校验已生成的 schema 文件与样例实例；当 schema 无效或样例实例不符合 schema 时 MUST 返回非零退出码。样例实例来源按以下优先级选择：

1) 对于 config schemas（全局/项目/llmanspec）：沿用既有策略：
   - 若对应配置文件存在，则使用真实文件内容（全局/项目/llmanspec）。
   - 若不存在，则使用默认配置实例作为样例。
   - 若对应配置文件存在但无法读取或无法解析为有效 YAML，命令 MUST 直接失败（不得静默回退 defaults）。
2) 对于 `llman x sdd-eval` playbook schema：
   - 命令 MUST 使用内置的“playbook 模板实例”（与 `llman x sdd-eval init` 模板语义一致）作为样例进行校验。
   - 校验 MUST 不依赖用户项目内是否存在 playbook 文件（避免在任意目录运行时产生不确定性）。

#### Scenario: Schema 校验失败
- **WHEN** `llman self schema check` 发现 schema 无效或样例实例不匹配
- **THEN** 命令返回非零退出码并报告错误

#### Scenario: 使用真实全局配置作为样例
- **WHEN** `LLMAN_CONFIG_DIR/config.yaml` 存在且用户运行 `llman self schema check`
- **THEN** 命令使用该文件内容作为样例并对照 schema 校验

#### Scenario: 真实配置不可读/不可解析会失败
- **WHEN** `LLMAN_CONFIG_DIR/config.yaml` 存在但无法读取或无法解析为有效 YAML
- **THEN** `llman self schema check` 返回非零错误且不会回退到默认实例作为样例
