## ADDED Requirements
### Requirement: 交互选择必须使用稳定对话标识符
交互式 cursor export MUST 将用户选择映射到稳定的对话标识符（例如：composer 使用 `composer_id`，传统聊天使用 `tab_id`）。导出结果 MUST 与被选择的标识符集合一致，而不是依赖展示 index。

#### Scenario: 排序展示不影响导出正确性
- **WHEN** 交互 UI 展示的是排序后的对话列表，用户选择了 A 与 B
- **THEN** 即使底层存储顺序不同，导出也只包含 A 与 B

### Requirement: 搜索选择必须共享同一标识符空间
交互搜索结果 MUST 返回与主列表相同的稳定标识符。导出 MUST 将搜索选择与主列表选择以一致方式处理（可合并、可去重，不得混用不同 index 空间）。

#### Scenario: 搜索选择导出正确
- **WHEN** 用户通过 “search more” 流程选择了若干对话
- **THEN** 导出结果与搜索选择完全一致

### Requirement: 交互导出不得加载未选中对话的完整内容
交互式 cursor export MUST 避免为未选中对话加载完整内容（尤其是 composer bubbles 等重数据），只为被选中项加载必要数据。

#### Scenario: 大库仅选少量对话
- **WHEN** workspace 中存在大量对话但用户只选择少量导出
- **THEN** 导出仅为被选中对话加载完整内容
