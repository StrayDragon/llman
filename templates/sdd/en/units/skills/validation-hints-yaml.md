<!-- llman-template-version: 1 -->
Validation fixes (YAML style):

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

2) Missing canonical ` ```yaml ` payload in a main spec (`llmanspec/specs/<feature-id>/spec.md`):
```yaml
kind: llman.sdd.spec
name: sample
purpose: 'One-line overview.'
requirements:
- req_id: r1
  title: Title
  statement: System MUST do something.
scenarios:
- req_id: r1
  id: happy
  given: ''
  when: a trigger happens
  then: the outcome is observed
```

3) No delta ops in a change: add at least one op + scenario in
`llmanspec/changes/<change-id>/specs/<feature-id>/spec.md`:
```yaml
kind: llman.sdd.delta
ops:
- op: add_requirement
  req_id: r1
  title: Title
  statement: System MUST do something.
  from: null
  to: null
  name: null
op_scenarios:
- req_id: r1
  id: happy
  given: ''
  when: a trigger happens
  then: the outcome is observed
```

Notes:
- YAML specs/deltas MUST be a single ` ```yaml ` fence per file.
- YAML write-back prefers comment/format-preserving overlay updates; when that fails, the CLI falls back to rewriting the fenced YAML payload deterministically (payload comments may be lost).
- `yaml` is experimental: prefer explicit `llman sdd convert` when migrating styles.
