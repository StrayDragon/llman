<!-- llman-template-version: 1 -->
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

### Notes
- `toon` uses explicit array headers like `requirements[<n>]{...}:` and tabular rows.
- `null` represents missing optional fields.
- Do not mix styles inside one project. Use `llman sdd convert` for explicit migrations.
