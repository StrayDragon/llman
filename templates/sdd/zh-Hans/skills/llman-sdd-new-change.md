---
name: "llman-sdd-new-change"
description: "创建新的 SDD 变更提案与增量 specs。"
metadata:
  llman-template-version: 1
---

# LLMAN SDD 新变更

当引入新能力、破坏性变更或架构调整时使用此 skill。

## 步骤
1. 选择唯一的 change id（kebab-case，动词前缀：`add-`、`update-`、`remove-`、`refactor-`）。
2. 创建 `llmanspec/changes/<change-id>/`，包含：
   - `proposal.md`
   - `tasks.md`
   - 可选 `design.md`
3. 对每个受影响能力创建 `specs/<capability>/spec.md`，并使用：
   - `## ADDED|MODIFIED|REMOVED|RENAMED Requirements`
   - 每条 requirement 至少一个 `#### Scenario:`
4. 校验：`llman sdd validate <change-id> --strict --no-interactive`。

{{region: templates/sdd/zh-Hans/skills/shared.md#sdd-commands}}

{{region: templates/sdd/zh-Hans/skills/shared.md#validation-hints}}

{{region: templates/sdd/zh-Hans/skills/shared.md#structured-protocol}}
{{region: templates/sdd/zh-Hans/skills/shared.md#future-planning}}
