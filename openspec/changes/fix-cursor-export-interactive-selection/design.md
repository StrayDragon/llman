## Context
当前交互导出流程存在“index 空间错配”：
1) 构建并排序 summaries 列表（展示用）。
2) 收集用户选择的 summaries index。
3) 再加载另一份未按同序排序的完整导出列表，并用 index 过滤。

这会导致导出结果与用户选择不一致。

## Goals / Non-Goals
- Goals:
  - 选择必须稳定、可去歧义，不依赖展示 index。
  - 搜索选择必须与主列表共享同一标识符空间，禁止混用不同 index。
  - 只加载被选中对话的完整内容，避免大库性能灾难。
- Non-Goals:
  - 不改变非交互模式的对外语义（除修复错误导出外）。
  - 不重做输出格式/文件命名/内容渲染。

## Decisions
- Decision: 引入统一的 `ConversationKey`（传统聊天优先 `tab_id`，composer 使用 `composer_id`），并构建统一的 `ConversationRef` 列表用于展示与导出。
  - Rationale: 彻底消除 index 错配，并为去歧义/去重提供基础。
- Decision: 展示 label 附带短 ID 后缀（例如 ID 前 8 位），避免同标题误选。

## Risks / Trade-offs
- 部分传统聊天可能缺失 `tab_id`；缓解：提供稳定 fallback key（例如派生 hash），并在 UI 中标记为 `no-id` 以提示不稳定性。

## Open Questions
- 对缺失 `tab_id` 的传统聊天，fallback key 的最佳策略：派生 key（可稳定） vs 强制共享排序并继续使用 index（实现简单但脆弱）。
