## Canonical TOON Spec Contract

SDD main specs and delta specs are authored as **standalone `.toon` files** — one TOON document per file, with no Markdown shell and no fenced code block. All structured information, including the validation proof-metadata (formerly a YAML frontmatter), lives inside the TOON document.

### Main spec (`llmanspec/specs/<feature-id>/spec.toon`)

```toon
kind: llman.sdd.spec
name: sample
purpose: "One-line overview."
valid_scope[2]: src/,tests/
requirements[1]{req_id,title,statement}:
  r1,Requirement title,System MUST do something.
scenarios[1]{req_id,id,given,when,then}:
  r1,happy,"",a trigger happens,the outcome is observed
```

- `kind` MUST be `llman.sdd.spec`.
- `name` SHOULD match the spec directory name.
- `valid_scope` is the validation scope (drives the staleness check). It MUST be present and non-empty, as a flat single-column tabular array (e.g. `valid_scope[2]: src/,tests/`). (`valid_commands` and `evidence` were dropped — only `valid_scope` is functionally consumed.)

### Main spec with BDD-on (Partitioned SSOT)

When `config.yaml` defines a `bdd` block, use **Partitioned SSOT**:

| Layer | Authority | Contents |
|---|---|---|
| Constraints | `spec.toon` | `requirements` + **non-executable** scenarios (`feature: false`) |
| Harness | `*.feature` | Executable GWT only; scenarios tagged `@req:<req_id>` |

```toon
kind: llman.sdd.spec
name: sample
purpose: "Constraints in toon; executable examples in .feature."
valid_scope[1]: llmanspec/specs/sample
requirements[1]{req_id,title,statement}:
  r1,New Requirement,System MUST do the new thing.
scenarios[1]{req_id,id,given,when,then,feature}:
  r1,internal-only,"manager scans","internal check","passes",false
```

```gherkin
# sample.feature
Feature: sample
  @req:r1
  Scenario: happy
    Given llman binary built
    When run llman sample --flag
    Then exit code 0
```

- **BDD-on (Git-native)**: edit live `.feature` and `spec.toon` on a non-default feature branch; bind with `llman sdd change attach`; prefer `change finalize` for single-commit close (or fallback: `checkpoint` then `change archive` before merge); `diff` is read-only review/export. Pre-merge archive/finalize moves change docs only — Git/PR merge promotes specs. Do **not** author `*.feature.delta.toon` (legacy active feature_delta is a migration blocker). There is no solidify command.
- **BDD-off**: use change-scoped TOON deltas (`ops` / `op_scenarios`) and archive merge as in the Delta section below — no attach/checkpoint/harness requirements.
- Downstream upgrade: `llman sdd project migrate --kind partitioned`.
- BDD enabled with empty `requirements` and no `.feature` is an ERROR.

### Delta spec (`llmanspec/changes/<change-id>/specs/<feature-id>/spec.toon`) — BDD-off / classic

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
- Migrate legacy `.md`+fence specs with `llman sdd migrate`.
