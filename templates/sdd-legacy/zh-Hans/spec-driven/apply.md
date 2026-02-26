<!-- llman-template-version: 1 -->
<!-- source: OpenSpec templates/zh-Hans/llman-sdd/apply.md (copied 2026-02-09) -->

为 `llmanspec/changes/<id>/` 的变更实施任务。

**输入**：可选在 `/llman-sdd:apply` 后指定变更 id（例如 `/llman-sdd:apply add-auth`）。如果省略，先从上下文推断；若不明确，必须让用户选择。

**步骤**

1. **选择变更**

   - 若用户提供了 id，直接使用。
   - 否则：
     - 若对话上下文明确指向某个变更 id，则使用它。
     - 否则运行 `llman sdd list --json`，展示最近修改的变更，让用户选择要实施哪一个。

   始终说明："使用变更：<id>"，并告知如何覆盖（例如 `/llman-sdd:apply <other>`）。

2. **检查前置条件**

   确保存在：
   - `llmanspec/changes/<id>/tasks.md`

   若缺失，建议先用 `/llman-sdd:continue <id>`（或 `/llman-sdd:ff <id>`）补齐规划工件，然后 STOP。

3. **阅读上下文工件**

   阅读：
   - `llmanspec/changes/<id>/proposal.md`（如果存在）
   - `llmanspec/changes/<id>/specs/*/spec.md`（所有 delta specs）
   - `llmanspec/changes/<id>/design.md`（如果存在）
   - `llmanspec/changes/<id>/tasks.md`

4. **展示当前进度**

   输出：
   - 进度："N/M tasks complete"
   - 接下来 1–3 个未完成任务（简短概览）

5. **按顺序实施任务（循环直到完成或受阻）**

   对每个未完成任务：
   - 说明正在处理哪一项任务
   - 做必要的代码修改（范围最小、聚焦）
   - 完成后立即在 `tasks.md` 勾选：`- [ ]` → `- [x]`

   遇到以下情况必须暂停：
   - 任务不清楚 → 先问用户再继续
   - 实现发现与 specs/design 不一致 → 建议先更新工件
   - 遇到错误/阻塞 → 汇报并请求指示

6. **全部完成后**

   当所有任务都勾选完成：
   - 建议 `/llman-sdd:verify <id>`（可选但推荐）
   - 建议 `/llman-sdd:archive <id>` 归档并更新主 specs

**护栏**
- 修改保持最小化，一次只专注一个任务
- 每完成一个任务就立刻更新 checkbox，保持进度真实

{{ unit("skills/structured-protocol") }}
