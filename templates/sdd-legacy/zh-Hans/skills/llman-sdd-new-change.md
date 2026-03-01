---
name: "llman-sdd-new-change"
description: "创建新的 SDD 变更提案与增量 specs。"
metadata:
  llman-template-version: 2
---

# LLMAN SDD 新变更

创建一个包含规划工件的变更（proposal + delta specs + tasks；design 可选）。

## 步骤
1. 明确 change id 与范围（kebab-case，动词前缀：`add-`、`update-`、`remove-`、`refactor-`）。
   - 若用户只给了描述，先问 1–3 个澄清问题，再提议一个 id 并让用户确认。
2. 确保项目已初始化：
   - 必须存在 `llmanspec/`；若不存在，提示先运行 `llman sdd-legacy init`，然后 STOP。
3. 创建 `llmanspec/changes/<change-id>/` 与 `llmanspec/changes/<change-id>/specs/`。
   - 若变更已存在，STOP 并建议使用 `llman-sdd-continue`（或 `/llman-sdd:continue <id>`）。
4. 在 `llmanspec/changes/<change-id>/` 下创建工件：
   - `proposal.md`（Why / What Changes / Capabilities / Impact）
   - 为每个 capability 创建 `specs/<capability>/spec.md`，并使用：
     - `## ADDED|MODIFIED|REMOVED|RENAMED Requirements`
     - 每条 requirement 至少一个 `#### Scenario:`
   - 仅在涉及权衡/迁移时创建 `design.md`
   - `tasks.md`：按顺序拆分为可勾选清单（包含校验命令）
5. 校验：`llman sdd-legacy validate <change-id> --strict --no-interactive`。
6. 进入实现阶段：建议使用 `llman-sdd-apply`（或 `/llman-sdd:apply <id>`）。

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}
