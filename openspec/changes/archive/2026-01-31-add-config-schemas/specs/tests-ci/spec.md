## ADDED Requirements
### Requirement: check-all 包含 schema 校验
`just check-all` MUST 包含 schema 校验步骤，确保生成的 JSON schema 与样例配置有效且可用。

#### Scenario: 运行 check-all
- **WHEN** 开发者运行 `just check-all`
- **THEN** `just check-schemas` 会被执行
