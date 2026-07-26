## Canonical TOON Spec Contract

SDD main specs are authored as **standalone `.toon` files** — one TOON document per file, with no Markdown shell and no fenced code block. All structured information, including the validation proof-metadata (formerly a YAML frontmatter), lives inside the TOON document.

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

### Partitioned SSOT (when `bdd:` is configured)

When `config.yaml` defines a `bdd` block, the `bdd:` section is a **runner-only** switch (`validate --check` runs `bdd.run_command`); it does **not** fork the change lifecycle. Use **Partitioned SSOT** for executable scenarios:

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

- **Git-native lifecycle**: edit live `.feature` and `spec.toon` on a non-default feature branch; enter Full stage with `llman sdd change start <id>` (recommended) or `change attach`; prefer `change finalize` for single-commit close-out (or fallback: `checkpoint` then `change archive`). Archive/finalize auto ff-merges the feature branch into the default branch, then renames change docs to `changes/archive/` (one follow-up `git commit` for the dirty rename). `diff` is read-only review/export. Do **not** author under `changes/<id>/specs/` or create `*.feature.delta.toon`. There is no `change delta`, solidify, or `llman-sdd-sync`.
- Downstream upgrade: manually remove leftover `change/specs/` or `*.feature.delta.toon` (`partitioned` migrate removed).
- `bdd:` enabled with empty `requirements` and no `.feature` is an ERROR.

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
