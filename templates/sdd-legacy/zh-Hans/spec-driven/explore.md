<!-- llman-template-version: 2 -->
<!-- source: OpenSpec src/core/templates/skill-templates.ts:getOpsxExploreCommandTemplate (copied 2026-02-09; adapted for llman) -->

进入 llman SDD 的探索模式：用于思考、调查与澄清（不实现）。

**重要：探索模式只用于思考（不实现）。**
- 你可以阅读文件、搜索代码、运行 `llman sdd-legacy` 命令。
- 只有在用户要求时，你才可以提议或起草 llman SDD 工件（proposal/specs/design/tasks）。
- 你绝对不能写应用代码或实现功能。

**输入**：用户想探索的任何主题（想法、问题、change id、对比选择，或不带参数）。

## 轻量流程

1. 澄清目标与约束（问 1–3 个问题）。
2. 若涉及具体 change id：
   - 运行 `llman sdd-legacy list --json` 确认其存在。
   - 阅读 `llmanspec/changes/<id>/` 下的工件（proposal/design/tasks/specs）。
3. 探索 2–3 个方案与权衡；必要时用简短 ASCII 图。
4. 当结论清晰时，建议把它记录下来（不要自动写）：
   - 范围变化 → `proposal.md`
   - 需求 → `llmanspec/changes/<id>/specs/<capability>/spec.md`
   - 决策 → `design.md`
   - 工作项 → `tasks.md`
5. 准备执行时，建议：
   - 开始 change：`/llman-sdd:new` 或 `llman-sdd-new-change`
   - 一次性创建工件：`/llman-sdd:ff`
   - 实施 tasks：`/llman-sdd:apply`

## 护栏
- 探索模式下绝不实现。
- 不要编造证据：以真实文件/命令输出为准。
- 若用户要求实现，STOP 并让其先退出探索模式（例如 `/llman-sdd:new` 或 `/llman-sdd:ff`）。

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}
