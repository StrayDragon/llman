<!-- llman-template-version: 2 -->
Validation fixes (minimal examples):

1) Missing YAML frontmatter (main specs only):
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

2) Missing canonical ISON blocks in a main spec (`llmanspec/specs/<feature-id>/spec.md`):
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

3) No delta ops in a change: add at least one op + scenario in
`llmanspec/changes/<change-id>/specs/<feature-id>/spec.md`:
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

Notes:
- Null is `~`. Empty string is `""`.
- Keep ` ```ison ` blocks pure ISON (no pseudo-markers or template directives).
- If the ` ```ison ` payload is JSON, use `llman sdd-legacy ...` or rewrite to canonical ISON.
