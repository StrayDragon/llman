<!-- llman-template-version: 1 -->
<!-- region: sdd-commands -->
Common commands:
- `llman sdd list` (list changes)
- `llman sdd list --specs` (list specs)
- `llman sdd show <id>` (show change/spec)
- `llman sdd validate <id>` (validate a change or spec)
- `llman sdd validate --all` (bulk validate)
- `llman sdd archive run <id>` (archive a change)
- `llman sdd archive <id>` (legacy alias of `archive run`)
- `llman sdd archive freeze [--before YYYY-MM-DD] [--keep-recent N] [--dry-run]` (freeze archived dirs into one cold-backup file)
- `llman sdd archive thaw [--change <id> ...] [--dest <path>]` (restore from cold-backup file)
<!-- endregion -->

<!-- region: opsx-quickstart -->
OPSX workflow:
- Install/update: `llman sdd update-skills --all`
- Claude Code command bindings: `.claude/commands/opsx/`
- Codex does not generate OPSX slash-command/custom-prompt bindings; use `llman-sdd-*` skills.

Common actions:
- `/opsx:new <id|description>` → create `llmanspec/changes/<id>/`
- `/opsx:continue <id>` → create the next artifact
- `/opsx:ff <id>` → create all artifacts quickly
- `/opsx:apply <id>` → implement tasks and update checkboxes
- `/opsx:verify <id>` → verify implementation vs artifacts
- `/opsx:archive <id>` → merge deltas + move to `llmanspec/changes/archive/`

Troubleshooting:
- If Claude `/opsx:*` is not recognized, rerun `llman sdd update-skills --all`.
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

<!-- region: structured-protocol -->
## Context
- Gather the current change/spec state before acting.

## Goal
- State the concrete outcome for this command/skill execution.

## Constraints
- Keep changes minimal and scoped.
- Avoid guessing when identifiers or intent are ambiguous.

## Workflow
- Use `llman sdd` commands as the source of truth.
- Validate outcomes when files or specs are updated.

## Decision Policy
- Ask for clarification when a high-impact ambiguity remains.
- Stop instead of forcing through known validation errors.

## Output Contract
- Summarize actions taken.
- Provide resulting paths and validation status.
<!-- endregion -->

<!-- region: future-planning -->
## Future-to-Execution Planning
- Treat `llmanspec/changes/<id>/future.md` as a candidate backlog, not passive notes.
- Review `Deferred Items`, `Branch Options`, and `Triggers to Reopen`; classify each item as:
  - `now` (must be converted into executable work now)
  - `later` (keep in future.md with explicit trigger/signal)
  - `drop` (remove or mark rejected with rationale)
- For each `now` item, propose a concrete landing path:
  - follow-up change id (`add-...`, `update-...`, `refactor-...`)
  - affected capability/spec path
  - first executable action (`/opsx:new`, `/opsx:continue`, `/opsx:ff`, or `llman-sdd-apply`)
- Keep traceability: reference source future item in the new proposal/design/tasks notes.
- When uncertainty is high, pause and ask before creating new change artifacts.
<!-- endregion -->
