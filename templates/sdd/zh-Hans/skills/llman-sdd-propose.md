---
name: "llman-sdd-propose"
description: "提出一个新变更并一次性生成规划工件。"
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD 提案（Propose）

创建一个新变更，并一次性生成所有规划工件（proposal + delta specs + tasks；design 可选），然后执行校验并建议下一步动作。

## 步骤
1. 判断变更规模（triage）：
   - **行为合约变更**（改 MUST/SHALL、改外部行为）→ 走完整 SDD 流程
   - **实现变更**（重构、typo、性能）→ 建议走快速路径，用 `llman-sdd-quick`
   - **元规范变更**（改 SDD 模板/流程）→ 必须走完整 SDD 流程
   - 不确定时走完整 SDD 流程（保守选择）
2. 使用 `llman sdd context --task "<目标>" --paths "<范围>"` 获取相关 specs。
   - 如果 context 不可用，运行 `llman sdd index rebuild`（默认 `pageindex`，无需模型）后继续。
3. 收集输入：
   - 变更的简要描述
   - change id（若未给出则推导；kebab-case，动词前缀：`add-`、`update-`、`remove-`、`refactor-`）
   - 受影响的 capability/capabilities（用于命名 `specs/<capability>/`）
   - 在写入任何文件前确认最终 id
2. 确保项目已初始化：
   - 必须存在 `llmanspec/`；若不存在，提示先运行 `llman sdd init`，然后 STOP。
3. 创建 `llmanspec/changes/<change-id>/` 与 `llmanspec/changes/<change-id>/specs/`。
   - 若变更已存在，STOP 并建议使用 `llman-sdd-continue`。
4. 在 `llmanspec/changes/<change-id>/` 下创建工件：
   - `proposal.md`（Why / What Changes / Capabilities / Impact）
   - 为每个 capability 创建 `specs/<capability>/spec.toon`（每个文件一份独立的 TOON 文档）：
     - 建议优先通过 authoring helpers 生成，确保 TOON payload 规范：
       - `llman sdd delta skeleton <change-id> <capability>`
       - `llman sdd delta add-op ...`
       - `llman sdd delta add-scenario ...`
     - 至少包含一个 `add_requirement`/`modify_requirement` op（statement 必须含 MUST/SHALL），并且至少包含一行匹配的 op scenario
   - 仅在涉及权衡/迁移时创建 `design.md`
   - `tasks.md`：按顺序拆分为可勾选清单（包含校验命令）
5. 校验：
   ```bash
   llman sdd validate <change-id> --strict --no-interactive
   ```
   此步骤必须通过后才能继续。若出现 TOON 解析错误，需修复引号：表格化行中包含逗号/冒号/方括号的值必须用双引号包裹。
6. 总结已创建内容，并建议使用 `llman-sdd-apply` 进入实现阶段。

{{ unit("skills/sdd-commands") }}
{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}
