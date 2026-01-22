## Why
- The current OpenSpec content lives under a custom `openspec/llman-cli-quality-uplift/` layout, so `openspec list` reports no active changes and `openspec list --specs` reports no specs.
- The CLI quality uplift work needs to be expressed in standard OpenSpec change/spec artifacts so the OpenSpec CLI can validate and track it.

## What Changes
- Create a standard OpenSpec change (`update-cli-quality-specs`) with proposal, tasks checklist, and spec deltas.
- Convert existing module docs into spec deltas for six capabilities: config-paths, errors-exit, cursor-export, tool-clean-comments, cli-experience, tests-ci.
- Preserve the existing project-level goals/constraints by moving them into `openspec/project.md`.
- Retire the custom `openspec/llman-cli-quality-uplift/` layout after migration.

## Impact
- Specs affected: config-paths, errors-exit, cursor-export, tool-clean-comments, cli-experience, tests-ci (new in this change).
- Code impact: none in this proposal phase (documentation only).
