<!-- region: sdd-commands -->
Common commands:
- `llman sdd list` (list changes)
- `llman sdd list --specs` (list specs)
- `llman sdd show <id>` (show change/spec)
- `llman sdd validate <id>` (validate a change or spec)
- `llman sdd validate --all` (bulk validate)
- `llman sdd archive <id>` (archive a change)
<!-- endregion -->

<!-- region: validation-hints -->
Validation fixes (minimal examples):

1) Missing `## Purpose` or `## Requirements`:
```markdown
## Purpose
<State the goal in one sentence>

## Requirements
### Requirement: <name>
The system MUST ...

#### Scenario: <happy path>
- **WHEN** ...
- **THEN** ...
```

2) Scenario header format:
```markdown
#### Scenario: <name>
- **WHEN** ...
- **THEN** ...
```

3) No delta in a change: add at least one requirement block in
`llmanspec/changes/<change-id>/specs/<capability>/spec.md`:
```markdown
## ADDED Requirements
### Requirement: <name>
The system MUST ...

#### Scenario: <name>
- **WHEN** ...
- **THEN** ...
```
<!-- endregion -->
