## 1. 修改 Skills 配置

- [x] 1.1 从 `src/skills/config/mod.rs` 的 `default_targets()` 中移除 `agent_global` ConfigEntry

## 2. 移除 Agent Display/Label 逻辑

- [x] 2.1 从 `display_agent_label()` 移除 `"agent" => "_agentskills_".to_string()` 分支
- [x] 2.2 从 `display_scope_label()` 移除 `("agent", "global") => "Global".to_string()` 分支
- [x] 2.3 从 `agent_order()` 移除 `"agent" => 2` 分支
- [x] 2.4 从 `scope_order()` 移除 `("agent", "global") => 0` 分支

## 3. 移除 Agent 选择流程特殊路径

- [x] 3.1 从 `src/skills/cli/command.rs` 移除 `if agent == "agent" && scope_choices.len() == 1` 特殊处理分支（约第 449-456 行）

## 4. 更新单元测试

- [x] 4.1 更新 `test_selectable_agents_labels` 中期望的 labels Vec，移除 `_agentskills_`
- [x] 4.2 移除 `test_scope_label_for_known_agent_and_scope` 中 `agent` / `global` 相关 assertion
- [x] 4.3 移除 `test_agent_scopes_filters_and_orders_expected_scopes` 中 `agent_global` 相关测试数据
- [x] 4.4 检查并移除其他引用 `agent` / `_agentskills_` 的测试

## 5. 验证

- [x] 5.1 运行 `just check` 确保 clippy / fmt / tests 通过
