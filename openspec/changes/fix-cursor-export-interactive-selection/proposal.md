## Why
交互式 Cursor export 目前可能导出“错误的对话内容”，属于高严重度正确性问题：
- 交互选择基于“展示列表的 index”，但最终导出过滤的是另一份未按同序排序的导出列表，index 空间不一致会导致错配导出。
- “搜索更多”返回的是搜索结果列表的 index，但上层把它当作主列表 index 使用，同样会错配。
- 展示 label 仅由标题/时间拼接，存在重复标题时误选风险。
- 性能上，哪怕只选少量对话，也会加载所有对话的完整内容（尤其 composer bubbles），大库会非常慢。

### Current Behavior（基于现有代码）
- 交互流程：`summaries` 按时间排序（`src/x/cursor/database.rs`），选择返回 summaries index（`src/x/cursor/command.rs`），随后对 `get_all_conversations_mixed()` 的枚举 index 做过滤（两者顺序不同）。
- 搜索：`search_conversations` 返回的是 search results 的 index（`src/x/cursor/command.rs`），被直接混入 summaries index 使用。

## What Changes
- 使用稳定标识符（composer 使用 `composer_id`，传统聊天优先使用 `tab_id`）作为交互选择与搜索选择的返回值，消除 index 错配。
- 导出阶段按“标识符集合”精确拉取与导出，确保导出内容与用户选择一致。
- 性能优化：只为被选中的对话加载完整内容（尤其 composer bubbles），避免对未选中项做重 IO/解析。

### Non-Goals（边界）
- 不更改非交互模式的输出格式与参数语义（除 bug 修正外）。
- 不引入新的输出模式或文件命名规则变更。

## Impact
- Affected specs: `specs/cursor-export/spec.md`
- Affected code:
  - `src/x/cursor/command.rs`（交互选择与映射）
  - `src/x/cursor/database.rs`（按 ID 拉取数据的 API）
  - `src/x/cursor/models.rs`（必要时补充选择 key）
