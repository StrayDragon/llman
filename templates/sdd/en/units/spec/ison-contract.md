<!-- llman-template-version: 1 -->
## Canonical ISON Spec Contract

All **new-style** SDD specs/deltas are authored as fenced ` ```ison ` blocks. A file may contain multiple ` ```ison ` fences; blocks are merged by `kind.name` and duplicates are an error.

### Main spec (`llmanspec/specs/<feature-id>/spec.md`)

Required canonical blocks and columns:

```ison
object.spec
kind name purpose
"llman.sdd.spec" sample "Describe the capability in one sentence."

table.requirements
req_id title statement
r1 "Requirement title" "System MUST do something."

table.scenarios
req_id id given when then
r1 happy "" "a trigger happens" "the outcome is observed"
```

### Delta spec (`llmanspec/changes/<change-id>/specs/<feature-id>/spec.md`)

Required canonical blocks and columns:

```ison
object.delta
kind
"llman.sdd.delta"

table.ops
op req_id title statement from to name
add_requirement r2 "New requirement" "System MUST do the new thing." ~ ~ ~

table.op_scenarios
req_id id given when then
r2 happy "" "the new trigger happens" "the new outcome is observed"
```

### Notes
- Quote strings with spaces/punctuation.
- Null is `~`. Empty string is `""` (for example: `given ""`).
- For `table.ops`, fields that are not applicable to the op MUST be `~`.
- Do **not** put pseudo-markers or templating directives inside ` ```ison ` blocks (keep them pure ISON).
