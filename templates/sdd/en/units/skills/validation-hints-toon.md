Validation fixes (TOON standalone specs):

1) Missing validation scope (`Spec valid_scope must not be empty`):
Main specs MUST carry a non-empty `valid_scope` inside the `.toon` document.
`llmanspec/specs/<feature-id>/spec.toon`:
```toon
kind: llman.sdd.spec
name: sample
purpose: "One-line overview."
valid_scope[1]: src
requirements[1]{req_id,title,statement}:
  r1,Title,System MUST do something.
scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
```

2) No delta ops in a change: add at least one op + scenario in
`llmanspec/changes/<change-id>/specs/<feature-id>/spec.toon`:
```toon
kind: llman.sdd.delta
ops[1]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,Title,System MUST do something.,null,null,null
op_scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
```

3) Tabular value quoting error ("Expected N tabular row values, but got M"):
Values containing **spaces**, commas, colons, or brackets MUST be double-quoted in tabular rows.
```toon
# BAD: spaces in an unquoted value split it into multiple values
r1,happy,"",a trigger happens,the outcome is observed

# GOOD: multi-word values quoted
r1,happy,"","a trigger happens","the outcome is observed"
```

4) BDD spec guardrail (Git-native Partitioned SSOT):
When `config.yaml` has a `bdd` block: `spec.toon` = constraints (requirements + non-executable scenarios); `*.feature` = executable harness GWT only (`@req:<req_id>`). Edit live specs/features on a non-default feature branch; bind with `llman sdd change attach`, gate with `checkpoint`, review with `diff` (read-only). Pre-merge `change archive` moves change docs only — never applies `feature_delta` / never merges TOON as SSOT. Legacy active `*.feature.delta.toon` is a migration blocker. Empty `requirements` with no `.feature` is an ERROR. There is no solidify command. Upgrade with `llman sdd project migrate --kind partitioned`.

Notes:
- Each spec is a single standalone `.toon` file; there is no Markdown shell or ```toon fence.
- `null` represents missing optional fields.
- Migrate legacy `.md`+fence specs with `llman sdd migrate`.
