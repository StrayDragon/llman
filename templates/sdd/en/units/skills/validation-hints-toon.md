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

2) Tabular value quoting error ("Expected N tabular row values, but got M"):
Values containing **spaces**, commas, colons, or brackets MUST be double-quoted in tabular rows.
```toon
# BAD: spaces in an unquoted value split it into multiple values
r1,happy,"",a trigger happens,the outcome is observed

# GOOD: multi-word values quoted
r1,happy,"","a trigger happens","the outcome is observed"
```

3) Git-native guardrail (Partitioned SSOT when `bdd:` configured):
`spec.toon` = constraints / non-executable scenarios; `*.feature` = executable GWT (`@req`). First `change start` or `change attach` (Branch binding), then edit live files on the bound non-default branch and commit (Specs landing) → prefer `change finalize` (single commit) or fallback `checkpoint` → ff-merge + docs rename via `change archive`. Do not use `change delta` / solidify / `*.feature.delta.toon`. Empty requirements with no `.feature` when `bdd:` is set = ERROR.

Notes:
- Each spec is a single standalone `.toon` file; there is no Markdown shell or ```toon fence.
- `null` represents missing optional fields.
- Migrate legacy `.md`+fence specs with `llman sdd migrate`.
