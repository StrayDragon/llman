## MODIFIED Requirements
### Requirement: SDD 模板版本元信息
SDD locale 模板 MUST 包含 `llman-template-version` 元信息。对于带 YAML frontmatter 的模板，frontmatter MUST 在 `metadata` 字段中包含 `llman-template-version` 键；其他模板 MUST 在第一行使用 `<!-- llman-template-version: N -->` 形式。相同相对路径的不同 locale 模板 MUST 使用相同的版本值。仓库 MUST 提供维护者检查命令以验证版本一致性与模板集合一致性。

#### Scenario: 模板版本一致性检查
- **WHEN** 维护者运行 `just check-sdd-templates`
- **THEN** 命令在缺失元信息、缺少 locale 模板或版本不一致时退出非零
- **AND** 在所有模板一致时退出零
