---
name: "llman-sdd-new-change"
description: "创建新变更提案与规划工件。"
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD New Change

创建一个新变更及规划工件（proposal + tasks；design 可选）。在 feature 分支上编辑 live specs。

## 步骤
1. 确定 change id 与范围（kebab-case，动词前缀：`add-`、`update-`、`remove-`、`refactor-`）。
   - 若用户只给了描述，先问 1–3 个澄清问题，再提议 id 并确认。
2. 确保项目已初始化：
   - 必须存在 `llmanspec/`；若不存在，提示先运行 `llman sdd init`，然后 STOP。
3. 创建 `llmanspec/changes/<change-id>/`（**不要**建 `specs/` 子目录）。
   - 若变更已存在，STOP 并建议使用 `llman-sdd-continue`。
4. 在 `llmanspec/changes/<change-id>/` 下创建工件：
   - `proposal.md`（Why / What Changes / Capabilities / Impact）
   - 仅在涉及权衡/迁移时创建 `design.md`
   - `tasks.md` 作为有序 checklist（含校验命令）
   - 在非默认 feature 分支上编辑 live `llmanspec/specs/<capability>/spec.toon`（配置了 `bdd:` 时再加带 `@req` 的 `*.feature`）；然后 `llman sdd change start <change-id>`（或 `change attach`）。**禁止**在 `changes/<id>/specs/` 下编写或创建 `*.feature.delta.toon`。
5. 校验：`llman sdd validate <change-id> --strict --no-interactive`。
   此步骤必须通过后才能继续。若出现 TOON 解析错误，需修复引号：表格化行中包含逗号/冒号/方括号的值必须用双引号包裹。
6. 交给实施：建议 `llman-sdd-apply`。

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}
