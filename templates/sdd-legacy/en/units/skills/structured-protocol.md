<!-- llman-template-version: 1 -->
<!-- legacy-track -->
## Context
- Gather the current change/spec state before acting.

## Goal
- State the concrete outcome for this command/skill execution.

## Constraints
- Keep changes minimal and scoped.
- Avoid guessing when identifiers or intent are ambiguous.

## Workflow
- Use `llman sdd-legacy` commands as the source of truth.
- Validate outcomes when files or specs are updated.

## Decision Policy
- Ask for clarification when a high-impact ambiguity remains.
- Stop instead of forcing through known validation errors.

## Output Contract
- Summarize actions taken.
- Provide resulting paths and validation status.
