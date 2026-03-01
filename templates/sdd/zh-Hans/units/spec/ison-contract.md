<!-- llman-template-version: 1 -->
## Canonical ISON Spec Contract

所有 **new-style** 的 SDD spec / delta 都以 fenced ` ```ison ` block 编写。一个文件中可以有多个 ` ```ison ` fence；会按 `kind.name` 合并，出现重复 block 名称会报错。

### Main spec（`llmanspec/specs/<feature-id>/spec.md`）

必须包含的 canonical blocks 与 columns：

```ison
object.spec
kind name purpose
"llman.sdd.spec" sample "用一句话描述能力的目的。"

table.requirements
req_id title statement
r1 "需求标题" "System MUST do something."

table.scenarios
req_id id given when then
r1 happy "" "发生触发条件" "观察到预期结果"
```

### Delta spec（`llmanspec/changes/<change-id>/specs/<feature-id>/spec.md`）

必须包含的 canonical blocks 与 columns：

```ison
object.delta
kind
"llman.sdd.delta"

table.ops
op req_id title statement from to name
add_requirement r2 "新增需求" "System MUST do the new thing." ~ ~ ~

table.op_scenarios
req_id id given when then
r2 happy "" "发生新的触发条件" "观察到新的预期结果"
```

### 备注
- 含空格/标点的字符串建议加引号。
- Null 用 `~` 表示；空字符串用 `""`（例如：`given ""`）。
- `table.ops` 中不适用于该 op 的字段 MUST 写成 `~`。
- ` ```ison ` block 内不要放伪标记或模板指令（保持纯 ISON）。
