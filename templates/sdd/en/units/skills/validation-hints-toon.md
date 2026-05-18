Validation fixes (TOON style):

1) Missing YAML frontmatter (main specs only):
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

2) Missing canonical ` ```toon ` payload in a main spec (`llmanspec/specs/<feature-id>/spec.md`):
```toon
kind: llman.sdd.spec
name: sample
purpose: "One-line overview."
requirements[1]{req_id,title,statement}:
  r1,Title,System MUST do something.
scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
```

3) No delta ops in a change: add at least one op + scenario in
`llmanspec/changes/<change-id>/specs/<feature-id>/spec.md`:
```toon
kind: llman.sdd.delta
ops[1]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,Title,System MUST do something.,null,null,null
op_scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
```

4) Tabular value quoting error ("Expected N tabular row values, but got M"):
Values containing commas, colons, or brackets MUST be double-quoted in tabular rows.
```toon
# BAD: commas in statement parsed as delimiters
r1,title,System MUST do X, Y, and Z.

# GOOD: quote the value containing commas
r1,title,"System MUST do X, Y, and Z."
```

Notes:
- `toon` specs/deltas MUST be a single ` ```toon ` fence per file.
- `null` represents missing optional fields.
- `toon` is experimental: prefer explicit `llman sdd convert` when migrating styles.
