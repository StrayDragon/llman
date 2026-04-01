## Why

我们已经完成 SDD new track 的验证与迁移，`sdd-legacy` 轨道只会带来双轨维护成本、提示词/校验语义漂移风险与用户困惑。当前目标是收敛为单一新技术栈与单一真源，彻底移除 legacy 相关机制与功能。

## What Changes

- 移除 `llman sdd-legacy ...` 命令组及其所有实现路径（包括 legacy 风格模板选择、legacy JSON-in-` ```ison ` 解析/校验语义、legacy-only 子命令与提示信息）。
- 删除 `templates/sdd-legacy/**` 模板目录，并移除模板渲染/生成中对 legacy track 的支持逻辑。
- 清理与 `sdd-legacy` 相关的文档、提示词、错误提示与测试用例；`llman sdd` 仅保留 new track 的 canonical table/object ISON 工作流。
- 更新 SDD eval / workflow DSL：移除 `sdd-legacy` style 与所有 legacy variants，只保留 new style 的可重复评测路径。
- 将 `openspec-propose` 迁移并适配为 `llman-sdd-propose`（面向 `llmanspec/`，使用 `llman sdd` 命令生成变更 proposal/specs/tasks 等工件），并纳入 `llman sdd update-skills --all` 的生成集合。

## Capabilities

### New Capabilities
- (none)

### Modified Capabilities
- `sdd-workflow`: 移除 legacy track/命令与 style 选择逻辑；收敛到单轨 new-style SDD。
- `sdd-legacy-compat`: 移除此 capability（或将其退役为历史说明），不再要求 legacy track 可用。
- `sdd-ison-pipeline`: 更新错误提示与校验约束，移除 “使用 `llman sdd-legacy ...`” 的建议路径。
- `sdd-structured-skill-prompts`: 移除 new/legacy 双轨要求与 legacy-command hint；确保新增 `llman-sdd-propose` 遵循结构化提示协议。
- `sdd-eval-acp-pipeline`: 移除 `sdd-legacy` style 评测与初始化逻辑，仅保留 new style。
- `sdd-eval-workflow-dsl`: 移除 DSL 中的 `sdd-legacy` style 与相关映射。

## Impact

- **BREAKING**：移除 `llman sdd-legacy ...` 及 legacy track 后，依赖该命令或 legacy 模板/语义的用户需要切换到 `llman sdd ...`（本变更假设仓库与团队已完成迁移）。
- 影响 `llman sdd` CLI 帮助文本、错误提示、模板生成与测试基线；CI 需要覆盖 `just check` 与 SDD 相关集成测试更新。
