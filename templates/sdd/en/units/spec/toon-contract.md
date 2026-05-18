## Canonical TOON Spec Contract (experimental)

In a `spec_style: toon` project, SDD main specs and delta specs are authored as **one** fenced ` ```toon ` code block containing one canonical TOON document.

### Main spec (`llmanspec/specs/<feature-id>/spec.md`)

```toon
kind: llman.sdd.spec
name: sample
purpose: "One-line overview."
requirements[1]{req_id,title,statement}:
  r1,Requirement title,System MUST do something.
scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
```

### Delta spec (`llmanspec/changes/<change-id>/specs/<feature-id>/spec.md`)

```toon
kind: llman.sdd.delta
ops[1]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,New requirement,System MUST do the new thing.,null,null,null
op_scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",the new trigger happens,the new outcome is observed
```

### Quoting Rules for Tabular Rows

In tabular array rows (values separated by commas), any value that contains a **comma**, **colon**, **bracket** (`[`, `]`, `{`, `}`), or starts/ends with whitespace **must be double-quoted**:

```
# BAD: commas in statement parsed as delimiters → field count mismatch
r1,title,System MUST do X, Y, and Z.

# GOOD: quote the value containing commas
r1,title,"System MUST do X, Y, and Z."
```

- Empty strings: `""`
- Optional fields not set: `null`
- When in doubt, quote the value.

### Notes
- `toon` uses explicit array headers like `requirements[<n>]{...}:` and tabular rows.
- `null` represents missing optional fields.
- Do not mix styles inside one project. Use `llman sdd convert` for explicit migrations.
