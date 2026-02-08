## Why

llman sdd 当前的 skills 系统仅提供基础 SKILL.md 生成，缺少 OpenSpec 风格的完整工作流支持（explore/continue/apply/ff/verify/sync）。用户需要一种平滑升级方式，让 llman sdd 具备完整流程指导，同时保持现有命令与命名前缀兼容。

## What Changes

- 扩展 skills 模板系统，新增 OpenSpec 风格工作流 skills：`explore`、`continue`、`apply`、`ff`、`verify`、`sync`
- 在 skills 模板中支持 `{{prompt: <name>}}` 占位符，注入可复用 prompt 片段
- 新增 `templates/sdd/<locale>/prompts/` 目录与项目级 prompts 覆盖机制
- 在 `llmanspec/config.yaml` 新增可选配置 `prompts.custom_path`，用于自定义 prompts 来源
- 保持 `llman-sdd-*` 命名前缀以确保兼容性
- 更新 `llman sdd update-skills` 命令以生成完整的工作流 skills 集合
- 明确 `llman-sdd-sync` 在 V1 提供“可复现的人工作业协议”指导，不引入自动 delta 合并引擎

## Out of Scope

- 本次不支持将 prompts 注入到 `AGENTS.md`
- 本次不实现自动化 delta specs 合并器（`sync` 仅提供流程化指导）
- 本次不修改 `llman skills`（skills-management 能力）

## Capabilities

### New Capabilities
- `sdd-prompts-injection`: 支持 prompt 模板注入到 sdd skills
- `sdd-workflow-skills`: 新增 OpenSpec 风格的工作流 skills（explore、continue、apply、ff、verify、sync）

### Modified Capabilities
- `sdd-workflow`: 扩展现有 sdd 工作流以支持 OpenSpec 模式

## Impact

- 受影响的 specs: `sdd-workflow`
- 受影响的代码:
- `src/sdd/project/templates.rs`: 扩展 skills 模板加载
- `src/sdd/project/regions.rs`: 扩展模板占位符解析（`{{prompt: ...}}`）
- `src/sdd/project/config.rs`: 新增 `prompts.custom_path` 配置
- `src/sdd/project/update_skills.rs`: 支持生成新的 skills
- `src/config_schema.rs`: `SddConfig` schema 生成路径保持一致并覆盖新字段
- `artifacts/schema/configs/en/llmanspec-config.schema.json`: 刷新 schema
- `templates/sdd/*/skills/`: 新增工作流 skills 模板
- `templates/sdd/*/prompts/`: 新增可注入 prompt 模板目录
