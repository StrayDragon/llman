## ADDED Requirements

### Requirement: Spec Authoring Prompts Include Canonical Table ISON Guidance
New-style SDD skills and spec-driven templates MUST include canonical table/object ISON guidance when they instruct users/agents to create or edit:
- `llmanspec/specs/<capability>/spec.md`, or
- `llmanspec/changes/<change-id>/specs/<capability>/spec.md`

MUST include explicit guidance for the canonical table/object ISON schema:
- the strictly fixed canonical block names
- the required columns for each table
- minimal valid examples
- common validation failures and how to fix them
- the token-friendly scenario encoding rules (`table.scenarios` / `table.op_scenarios` use `given/when/then` columns; each value is a single quoted string; `\n` for newlines; `given` MAY be `""`)

Templates MUST NOT include invalid pseudo-markers inside fenced ` ```ison ` blocks (for example, `<meta-directives>` lines that are not valid `kind.name` headers).

Guidance MAY be provided either:
- inline in the relevant template/skill, or
- via a globally injected llmanspec-managed “ISON spec contract” section (for example in `llmanspec/AGENTS.md`) that templates can reference.

#### Scenario: Template guidance matches the canonical schema
- **WHEN** a maintainer reviews `templates/sdd/{locale}/skills/*.md` and `templates/sdd/{locale}/spec-driven/*.md`
- **THEN** any guidance that references writing `spec.md` files uses canonical table/object ISON examples
- **AND** the guidance does not instruct Markdown heading-based delta sections like `## ADDED|MODIFIED|REMOVED|RENAMED Requirements` for llmanspec delta specs

#### Scenario: Generated skills include the ISON authoring guidance
- **WHEN** a user runs `llman sdd update-skills --no-interactive --all`
- **THEN** the generated `SKILL.md` content includes the canonical ISON authoring guidance for spec/delta creation where applicable

#### Scenario: Validation errors point to legacy command
- **WHEN** a user follows the templates and encounters an error because legacy JSON payloads are present
- **THEN** the guidance and/or error output includes a concrete legacy-command hint when appropriate (for example, `llman sdd-legacy validate ...`)
