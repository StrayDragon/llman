---
name: "llman-sdd-new-change"
description: "创建新的 SDD 变更提案与增量 specs。"
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD 新变更

创建一个包含规划工件的变更（proposal + delta specs + tasks；design 可选）。

## 步骤
1. 明确 change id 与范围（kebab-case，动词前缀：`add-`、`update-`、`remove-`、`refactor-`）。
   - 若用户只给了描述，先问 1–3 个澄清问题，再提议一个 id 并让用户确认。
2. 确保项目已初始化：
   - 必须存在 `llmanspec/`；若不存在，提示先运行 `llman sdd init`，然后 STOP。
3. 创建 `llmanspec/changes/<change-id>/` 与 `llmanspec/changes/<change-id>/specs/`。
   - 若变更已存在，STOP 并建议使用 `llman-sdd-continue`。
4. 在 `llmanspec/changes/<change-id>/` 下创建工件：
   - `proposal.md`（Why / What Changes / Capabilities / Impact）
   - 为每个 capability 创建 `specs/<capability>/spec.toon`（约束层；每个文件一份独立的 TOON 文档）：
     - 建议优先通过 authoring helpers 生成，确保 TOON payload 规范：
       - `llman sdd delta skeleton <change-id> <capability>`
       - `llman sdd delta add-op ...`
       - `llman sdd delta add-scenario ...`（仅约束层 / `feature:false`）
     - 至少包含一个 `add_requirement`/`modify_requirement` op（statement 必须含 MUST/SHALL），并且至少包含一行匹配的 op scenario
   - **BDD-on Partitioned**：可执行 GWT 写入 `specs/<capability>/*.feature.delta.toon`（或直接编辑主 `*.feature`），**不要**把完整 Given/When/Then 写进 toon 再靠 solidify 投影
   - 仅在涉及权衡/迁移时创建 `design.md`
   - `tasks.md`：按顺序拆分为可勾选清单（包含校验命令）
5. 校验：`llman sdd validate <change-id> --strict --no-interactive`。
   此步骤必须通过后才能继续。若出现 TOON 解析错误，需修复引号：表格化行中包含逗号/冒号/方括号的值必须用双引号包裹。
6. 进入实现阶段：建议使用 `llman-sdd-apply`。

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}
