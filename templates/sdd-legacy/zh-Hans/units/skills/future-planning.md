<!-- llman-template-version: 1 -->
## Future 到执行的规划
- 将 `llmanspec/changes/<id>/future.md` 视为候选待办池，而不是静态备注。
- 审查 `Deferred Items`、`Branch Options`、`Triggers to Reopen`，并把每项归类为：
  - `now`（需要立即转化为可执行工作）
  - `later`（保留在 future.md，补充明确触发信号）
  - `drop`（移除或标记拒绝并说明原因）
- 对每个 `now` 项，产出明确落地路径：
  - 后续 change id（`add-...`、`update-...`、`refactor-...`）
  - 受影响 capability/spec 路径
  - 第一条可执行动作（`/llman-sdd:new`、`/llman-sdd:continue`、`/llman-sdd:ff` 或 `llman-sdd-apply`）
- 保持可追溯性：在新 proposal/design/tasks 中引用来源 future 条目。
- 若存在高不确定性，先暂停并提问，再创建新变更工件。
