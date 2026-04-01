<!-- llman-template-version: 2 -->
# LLMAN Spec-Driven Development (SDD)

These instructions are for AI assistants working in this repository.

When a request:
- mentions proposal/spec/change/plan
- introduces a new feature, breaking change, architecture shift, or large performance/security work
- is ambiguous and needs authoritative specs

Use llmanspec and the `llman sdd` workflow.

Quick commands:
- `llman sdd list`
- `llman sdd show <item>`
- `llman sdd validate <id> --strict --no-interactive`
- `llman sdd archive <id>`
- `llman sdd update-skills --all`

Project context:
- `llmanspec/project.md` captures conventions and constraints.

Locale + skills:
- `llmanspec/config.yaml` sets `locale` and skills paths.
- Locale affects templates and skills only; CLI output stays English.
- Regenerate skills with `llman sdd update-skills`.
- The SDD workflow is single-track: use `llman sdd ...` (canonical table/object ISON only).

Only use AGENTS.md context injection.

Workflow prompts:
- Use the generated `llman-sdd-*` skills (regenerate with `llman sdd update-skills`).
- Keep workflow prompts centralized in skills templates; do not hand-maintain separate wrappers.

## Phase 1: Create a change
Create a proposal when:
- new capability or feature
- breaking change (API, schema)
- architecture or pattern shift
- performance/security work that changes behavior

Skip proposals for:
- bug fixes restoring expected behavior
- typos/formatting/comments
- non-breaking dependency updates
- config-only changes

Workflow:
1. Read `llmanspec/project.md`.
2. Check current state: `llman sdd list` and `llman sdd list --specs`.
3. Choose a unique change id: kebab-case, verb prefix (`add-`, `update-`, `remove-`, `refactor-`).
4. Create `llmanspec/changes/<change-id>/` with `proposal.md`, `tasks.md`, and optional `design.md`.
5. For each affected capability, add `llmanspec/changes/<change-id>/specs/<capability>/spec.md` using:
   - Canonical ISON blocks: `object.delta` + `table.ops` + `table.op_scenarios`
   - Use `~` for nulls and `""` for empty strings (for example: `given ""`)
6. Validate: `llman sdd validate <change-id> --strict --no-interactive`.

## Phase 2: Implement
Track these steps as TODOs and complete them in order.
1. Read `proposal.md`.
2. Read `design.md` if present.
3. Read `tasks.md`.
4. Implement tasks in order.
5. Update `tasks.md` checkboxes only when done.
6. Do not implement before proposal approval.

## Phase 3: Archive
After deployment:
- Run `llman sdd archive <change-id>`.
- Use `--skip-specs` for tooling-only changes.
- Validate again: `llman sdd validate --strict --no-interactive`.

## Spec authoring essentials
- Main specs live at `llmanspec/specs/<feature-id>/spec.md` and MUST include required YAML frontmatter keys:
  - `llman_spec_valid_scope`
  - `llman_spec_valid_commands`
  - `llman_spec_evidence`
- Delta specs live at `llmanspec/changes/<change-id>/specs/<feature-id>/spec.md` and MUST NOT include YAML frontmatter.
- Both spec types are authored as canonical table/object ISON blocks:

{{ unit("spec/ison-contract") }}

Keep this managed block so `llman sdd update` can refresh it.
