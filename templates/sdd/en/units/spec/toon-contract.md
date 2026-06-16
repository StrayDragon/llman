## Canonical TOON Spec Contract

SDD main specs and delta specs are authored as **standalone `.toon` files** — one TOON document per file, with no Markdown shell and no fenced code block. All structured information, including the validation proof-metadata (formerly a YAML frontmatter), lives inside the TOON document.

### Main spec (`llmanspec/specs/<feature-id>/spec.toon`)

```toon
kind: llman.sdd.spec
name: sample
purpose: "One-line overview."
valid_scope[2]:
  src/
  tests/
valid_commands[1]:
  "llman sdd validate sample --type spec --strict --no-interactive"
evidence[1]:
  "TODO: add evidence (CI link, benchmark output, etc.)"
requirements[1]{req_id,title,statement}:
  r1,Requirement title,System MUST do something.
scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
```

- `kind` MUST be `llman.sdd.spec`.
- `name` SHOULD match the spec directory name.
- `valid_scope` / `valid_commands` / `evidence` are the validation proof-metadata. Each MUST be present and non-empty. They are flat single-column tabular arrays (e.g. `valid_scope[2]: src/,tests/`).

### Main spec with BDD feature_refs (point-only)

When `config.yaml` defines a `bdd` block, a spec may point to `.feature` files for executable BDD. In **point-only** mode (bdd enabled + non-empty `feature_refs`) the behavior lives in the `.feature` file, so `requirements`/`scenarios` MAY be omitted entirely:

```toon
kind: llman.sdd.spec
name: sample
purpose: "Behavior lives in the referenced .feature file."
valid_scope[2]:
  src/
  tests/
valid_commands[1]:
  "pytest tests/features/sample.feature -v"
evidence[1]:
  "covered by .feature"
feature_refs[1]{path,scope,required}:
  tests/features/sample.feature,acceptance,true
```

- `path`: relative path to `.feature` file from project root
- `scope`: `acceptance` | `unit` | `reference`
- `required`: `true` → ERROR if missing; `false` → WARNING if missing
- If `requirements` are present they are still validated (statements MUST contain SHALL/MUST), but the "every requirement needs a scenario" rule is relaxed.
- A spec with BDD enabled, no `feature_refs`, and no `requirements` is an ERROR (make an explicit choice).

### Delta spec (`llmanspec/changes/<change-id>/specs/<feature-id>/spec.toon`)

```toon
kind: llman.sdd.delta
ops[1]{op,req_id,title,statement,from,to,name}:
  add_requirement,r1,New requirement,System MUST do the new thing.,null,null,null
op_scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",the new trigger happens,the new outcome is observed
```

- `kind` MUST be `llman.sdd.delta`.
- Delta specs carry no validation meta (only main specs do).

### Quoting Rules for Tabular Rows

In tabular array rows (values separated by commas), any value containing a **space**, **comma**, **colon**, **bracket** (`[`, `]`, `{`, `}`), or starts/ends with whitespace **must be double-quoted**:

```
# BAD: spaces in an unquoted value split it into multiple values
r1,happy,"",a trigger happens,the outcome is observed

# GOOD: multi-word values quoted
r1,happy,"","a trigger happens","the outcome is observed"
```

- Empty strings: `""`
- Optional fields not set: `null`
- When in doubt, quote the value.

### Notes
- One `.toon` file per spec; no Markdown, no ```` ```toon ```` fence.
- `null` represents missing optional fields.
- Migrate legacy `.md`+fence specs with `llman sdd convert`.
