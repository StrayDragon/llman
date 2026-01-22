## Context
The repository contains a CLI quality uplift spec in a custom `openspec/llman-cli-quality-uplift/` layout. OpenSpec CLI commands currently report no active changes or specs. The goal is to align the existing material with the standard OpenSpec structure without introducing new behavior changes.

## Goals / Non-Goals
- Goals:
  - Align CLI quality uplift documentation with OpenSpec change/spec format.
  - Preserve the existing scope, priorities, and acceptance criteria.
  - Provide traceable tasks and validation via OpenSpec CLI.
- Non-Goals:
  - No code or behavior changes in this proposal phase.
  - No additional capabilities beyond the documented quality uplift scope.

## Decisions
- Decision: Use change-id `update-cli-quality-specs` and create spec deltas under `openspec/changes/update-cli-quality-specs/specs/`.
  - Alternatives considered: Reuse the custom layout (rejected because OpenSpec CLI cannot validate it).
- Decision: Map existing module docs to six capabilities: config-paths, errors-exit, cursor-export, tool-clean-comments, cli-experience, tests-ci.
  - Alternatives considered: Merge into fewer specs (rejected to keep scope clear and traceable).
- Decision: Record English-only message requirements (LLMAN_LANG reserved) in `cli-experience` to reduce localization maintenance.
  - Alternatives considered: Multi-language specs (rejected for now due to stated constraints).

## Risks / Trade-offs
- Risk: Some requirements may not exactly match current implementation details.
  - Mitigation: Tie requirements to current code behavior and update deltas if gaps are found.
- Risk: Removing the custom OpenSpec layout may confuse readers with old links.
  - Mitigation: Use a single canonical change under `openspec/changes/` and update references if needed.

## Migration Plan
1. Move project-level overview to `openspec/project.md`.
2. Create `openspec/changes/update-cli-quality-specs/` with proposal, tasks, and spec deltas.
3. Remove the legacy `openspec/llman-cli-quality-uplift/` folder once content is migrated.
4. Validate the change with `openspec validate update-cli-quality-specs --strict --no-interactive`.

## Open Questions
- None.
