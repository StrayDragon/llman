<!-- llman-template-version: 2 -->
校验修复最小示例：

1) 缺少 YAML frontmatter（仅 main specs 需要）：
```markdown
---
llman_spec_valid_scope:
  - src
llman_spec_valid_commands:
  - cargo test
llman_spec_evidence:
  - <evidence>
---
```

2) Main spec 缺少 canonical ISON blocks（`llmanspec/specs/<feature-id>/spec.md`）：
```ison
object.spec
kind name purpose
"llman.sdd.spec" sample "用一句话概述。"

table.requirements
req_id title statement
r1 "标题" "System MUST do something."

table.scenarios
req_id id given when then
r1 happy "" "发生触发条件" "观察到预期结果"
```

3) Change 中没有 delta ops：至少在
`llmanspec/changes/<change-id>/specs/<feature-id>/spec.md` 添加一条 op + 场景：
```ison
object.delta
kind
"llman.sdd.delta"

table.ops
op req_id title statement from to name
add_requirement r1 "标题" "System MUST do something." ~ ~ ~

table.op_scenarios
req_id id given when then
r1 happy "" "发生触发条件" "观察到预期结果"
```

备注：
- Null 用 `~`；空字符串用 `""`。
- ` ```ison ` block 内保持纯 ISON（不要放伪标记或模板指令）。
- 如果 ` ```ison ` payload 是 JSON，请使用 `llman sdd-legacy ...` 或手工改写为 canonical ISON。
