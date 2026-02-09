<!-- llman-template-version: 1 -->
<!-- region: sdd-commands -->
Common commands:
- `llman sdd list` (list changes)
- `llman sdd list --specs` (list specs)
- `llman sdd show <id>` (show change/spec)
- `llman sdd validate <id>` (validate a change or spec)
- `llman sdd validate --all` (bulk validate)
- `llman sdd archive <id>` (archive a change)
<!-- endregion -->

<!-- region: opsx-quickstart -->
OPSX workflow (slash commands):
- Install/update: `llman sdd update-skills --all`
- Claude Code binds commands at: `.claude/commands/opsx/`
- Codex binds prompts at: `.codex/prompts/`

Common actions:
- `/opsx:new <id|description>` → create `llmanspec/changes/<id>/`
- `/opsx:continue <id>` → create the next artifact
- `/opsx:ff <id>` → create all artifacts quickly
- `/opsx:apply <id>` → implement tasks and update checkboxes
- `/opsx:verify <id>` → verify implementation vs artifacts
- `/opsx:archive <id>` → merge deltas + move to `llmanspec/changes/archive/`

Troubleshooting:
- If `/opsx:*` is not recognized, rerun `llman sdd update-skills --all`.
- If legacy bindings exist (`.claude/commands/openspec/` or `.codex/prompts/openspec-*.md`), rerun `llman sdd update-skills` in an interactive terminal to migrate (it requires double confirmation).
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
