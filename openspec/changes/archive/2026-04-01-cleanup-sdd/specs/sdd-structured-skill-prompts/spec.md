# sdd-structured-skill-prompts Specification (Delta)

## MODIFIED Requirements

### Requirement: 生成提示不得包含占位块或无效引导
SDD 的技能渲染结果 MUST 不包含会诱导不稳定行为的占位块或无效引导（例如 “Options / <option …> / What would you like to do?”）。

#### Scenario: update-skills 产物无占位块
- **WHEN** 维护者在同一代码版本下运行 `llman sdd update-skills --no-interactive --tool codex`
- **THEN** 生成的任意 `SKILL.md` 不包含子串 `Options:` 或 `<option`
- **AND** 生成的任意 `SKILL.md` 不包含子串 `What would you like to do?`

### Requirement: Spec Authoring Prompts Include Canonical Table ISON Guidance
New-style SDD skills MUST include canonical table/object ISON guidance when they instruct users/agents to create or edit:
- `llmanspec/specs/<capability>/spec.md`, or
- `llmanspec/changes/<change-id>/specs/<capability>/spec.md`

MUST include explicit guidance for the canonical table/object ISON schema:
- the strictly fixed canonical block names
- the required columns for each table
- minimal valid examples
- common validation failures and how to fix them
- the token-friendly scenario encoding rules (`table.scenarios` / `table.op_scenarios` use `given/when/then` columns; values are ISON strings; use double quotes when quoting is required; `\n` for newlines; `given` MAY be `""`)

Templates MUST NOT include invalid pseudo-markers inside fenced ` ```ison ` blocks (for example, `<meta-directives>` lines that are not valid `kind.name` headers).

Guidance MAY be provided either:
- inline in the relevant template/skill, or
- via a globally injected llmanspec-managed “ISON spec contract” section (for example in `llmanspec/AGENTS.md`) that templates can reference.

#### Scenario: Template guidance matches the canonical schema
- **WHEN** a maintainer reviews `templates/sdd/{locale}/skills/*.md`
- **THEN** any guidance that references writing `spec.md` files uses canonical table/object ISON examples
- **AND** the guidance does not instruct Markdown heading-based delta sections like `## ADDED|MODIFIED|REMOVED|RENAMED Requirements` for llmanspec delta specs

#### Scenario: Generated skills include the ISON authoring guidance
- **WHEN** a user runs `llman sdd update-skills --no-interactive --all`
- **THEN** the generated `SKILL.md` content includes the canonical ISON authoring guidance for spec/delta creation where applicable

#### Scenario: Validation errors include canonical rewrite guidance
- **WHEN** a user follows the templates and encounters an error because legacy JSON-in-` ```ison ` payloads are present
- **THEN** the guidance and/or error output includes a concrete hint to rewrite the payload into canonical table/object ISON (no legacy-command hint)
