## Why

Current SDD templates and generated skills are effective but increasingly hard to evolve for ethics-critical prompts, compact spec authoring, and measurable quality improvements. We need a default-new pipeline that improves structure and governance while keeping a safe fallback path for users.

## What Changes

- Introduce an ISON-first SDD pipeline for template authoring and generation, with Markdown as rendered compatibility output.
- Add strong ethics governance blocks to SDD prompt/skill protocol with explicit risk level, refusal contract, escalation policy, and required evidence fields.
- Add an A/B evaluation mechanism to compare legacy vs new styles before and during rollout.
- Keep legacy templates/skills/prompts available under a dedicated legacy track.
- Make the new style the default for SDD update/update-skills/show/validate flows, while allowing explicit legacy selection.
- Refactor spec parsing/validation/archive merge from Markdown heading conventions to ISON container semantics (`spec.md` with ` ```ison ` payload).
- Introduce one-shot migration workflow for existing SDD specs/delta specs so runtime check/merge operates only on ISON semantics.

## Capabilities

### New Capabilities
- `sdd-ison-pipeline`: Define ISON-first template source and rendering/validation workflow for SDD artifacts and skills.
- `sdd-legacy-compat`: Define dual-track compatibility behavior and explicit legacy selection paths.
- `sdd-ab-evaluation`: Define built-in A/B evaluation flow and scoring outputs for comparing old/new style quality.

### Modified Capabilities
- `sdd-workflow`: Extend SDD commands and generation flow to support default-new style and optional legacy style.
- `sdd-structured-skill-prompts`: Upgrade structured protocol to include strong ethics governance fields and enforcement.
- `sdd-specs-compaction-guidance`: Align compaction workflow with ISON-first source and measurable quality checks.
- `sdd-ison-pipeline`: Expand from template authoring to include ISON-based parsing, validation, and archive merge contracts for `llmanspec/specs/**` and `llmanspec/changes/**/specs/**`.

## Impact

- **Code paths**: `src/sdd/project/templates.rs`, `src/sdd/project/update.rs`, `src/sdd/project/update_skills.rs`, `src/sdd/shared/show.rs`, `src/sdd/shared/validate.rs`, CLI argument wiring in `src/sdd/command.rs`.
- **Spec engine**: `src/sdd/spec/parser.rs`, `src/sdd/spec/validation.rs`, `src/sdd/change/delta.rs`, `src/sdd/change/archive.rs`, and shared ISON parser utilities.
- **Templates**: add ISON source trees for `en` and `zh-Hans`; add legacy track templates; keep parity checks.
- **Tests**: expand SDD integration tests for format/style routing, legacy compatibility, and A/B reporting shape.
- **Docs/UX**: update SDD instructions to explain default new style, legacy override flags, and evaluation workflow.
