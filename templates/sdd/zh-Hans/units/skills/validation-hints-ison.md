<!-- llman-template-version: 1 -->
常见校验修复（ISON 风格）：

1) Main spec 缺少 YAML frontmatter（仅 main spec 需要）：
```markdown
---
llman_spec_valid_scope:
  - src
llman_spec_valid_commands:
  - llman sdd validate <feature-id> --type spec --strict --no-interactive
llman_spec_evidence:
  - <evidence>
---
```

2) Main spec 缺少 canonical ` ```ison ` blocks（`llmanspec/specs/<feature-id>/spec.md`）：
```ison
object.spec
kind name purpose
"llman.sdd.spec" sample "One-line overview."

table.requirements
req_id title statement
r1 "Title" "System MUST do something."

table.scenarios
req_id id given when then
r1 happy "" "a trigger happens" "the outcome is observed"
```

3) Change 缺少 delta ops：至少补一个 op + scenario（`llmanspec/changes/<change-id>/specs/<feature-id>/spec.md`）：
```ison
object.delta
kind
"llman.sdd.delta"

table.ops
op req_id title statement from to name
add_requirement r1 "Title" "System MUST do something." ~ ~ ~

table.op_scenarios
req_id id given when then
r1 happy "" "a trigger happens" "the outcome is observed"
```

备注：
- Null 用 `~`；空字符串用 `""`。
- ` ```ison ` block 内保持纯 ISON（不要放伪标记或模板指令）。
- 如果 ` ```ison ` payload 是 JSON，请手工改写为 canonical table/object ISON（main spec 使用 `object.spec` + `table.requirements` + `table.scenarios`；delta spec 使用 `object.delta` + `table.ops` + `table.op_scenarios`）。
