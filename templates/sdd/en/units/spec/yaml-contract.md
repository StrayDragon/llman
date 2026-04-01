<!-- llman-template-version: 1 -->
## Canonical YAML Spec Contract (experimental)

In a `spec_style: yaml` project, SDD main specs and delta specs are authored as **one** fenced ` ```yaml ` code block containing one canonical YAML document.

### Main spec (`llmanspec/specs/<feature-id>/spec.md`)

```yaml
kind: llman.sdd.spec
name: sample
purpose: 'One-line overview.'
requirements:
- req_id: r1
  title: Requirement title
  statement: System MUST do something.
scenarios:
- req_id: r1
  id: happy
  given: ''
  when: a trigger happens
  then: the outcome is observed
```

### Delta spec (`llmanspec/changes/<change-id>/specs/<feature-id>/spec.md`)

```yaml
kind: llman.sdd.delta
ops:
- op: add_requirement
  req_id: r1
  title: New requirement
  statement: System MUST do the new thing.
  from: null
  to: null
  name: null
op_scenarios:
- req_id: r1
  id: happy
  given: ''
  when: the new trigger happens
  then: the new outcome is observed
```

### Notes
- YAML write-back prefers comment/format-preserving overlay updates when possible.
- When a lossless update is not possible, the CLI falls back to rewriting the fenced YAML payload deterministically (comments inside the payload may be lost).
- Do not mix styles inside one project. Use `llman sdd convert` for explicit migrations.
