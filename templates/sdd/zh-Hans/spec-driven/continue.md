<!-- llman-template-version: 3 -->
<!-- source: OpenSpec templates/zh-Hans/llman-sdd/continue.md (copied 2026-02-09) -->

通过创建下一个工件来继续处理某个变更（位于 `llmanspec/changes/<id>/`）。

**输入**：可选在 `/llman-sdd:continue` 后指定变更 id（例如 `/llman-sdd:continue add-auth`）。如果省略，先从上下文推断；若不明确，必须让用户选择要继续的变更。

**步骤**

1. **选择要继续的变更**

   - 如果用户提供了变更 id，直接使用。
   - 否则：
     - 若对话上下文明确指向单个变更 id，则使用它。
     - 否则运行 `llman sdd list --json`，展示最近修改的 3–4 个变更，让用户选择继续哪一个。
   始终说明："使用变更：<id>"，并告知如何覆盖（例如 `/llman-sdd:continue <other>`）。

2. **确认变更存在**

   确认目录存在：`llmanspec/changes/<id>/`。
   - 若不存在：建议先运行 `/llman-sdd:new <id>`，然后 STOP。

3. **确定下一步要创建的工件（spec-driven）**

   默认采用 spec-driven 顺序：
   1) `proposal.md`
   2) `specs/<capability>/spec.md`（一次只做一个 capability）
   3) `design.md`（推荐但可选）
   4) `tasks.md`

   通过检查 `llmanspec/changes/<id>/` 下的文件是否存在来判断缺失项。

4. **只创建 ONE 个工件**

   - 如果缺少 `proposal.md`：创建它（Why / What Changes / Capabilities / Impact）。
   - 否则如果 `specs/*/spec.md` 还不存在：
     - 询问第一个 capability id（kebab-case），或从 proposal 的 Capabilities 中派生。
     - 创建 `llmanspec/changes/<id>/specs/<capability>/spec.md`，使用 canonical ISON（`object.delta` + `table.ops` + `table.op_scenarios`）。
       - 至少包含一个 `add_requirement`/`modify_requirement` op（statement 必须含 MUST/SHALL），并且至少包含一行匹配的 `table.op_scenarios`。
       - 参考 `llmanspec/AGENTS.md` 中的 Canonical ISON Spec Contract。
   - 否则如果缺少 `design.md` 且此变更需要设计说明（跨多个系统/复杂权衡/破坏性变更等）：
     - 创建 `design.md`，记录关键决策与理由。
   - 否则如果缺少 `tasks.md`：
     - 创建 `tasks.md`，用可勾选的有序小任务列表（包含验证命令）。
   - 否则：
     - 规划工件已齐全。建议使用 `/llman-sdd:apply <id>` 开始实施，或在准备好后用 `/llman-sdd:archive <id>` 归档，然后 STOP。

5. **建议运行校验**

   - 若至少存在一个 delta spec：建议运行 `llman sdd validate <id> --strict --no-interactive`。
   - 否则：解释 change 校验会在没有 delta spec 时失败（这是预期行为）。

**输出**

每次调用后，输出：
- 本次创建了哪个工件及其路径
- 下一步还缺什么
- 提示："运行 `/llman-sdd:continue <id>` 创建下一个工件"

**护栏**
- 每次只创建一个工件
- 写之前先读已有工件
- 任何不明确之处先问清楚再写
- continue 模式不要实现应用代码

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}
