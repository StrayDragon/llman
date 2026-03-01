<!-- llman-template-version: 2 -->
<!-- source: OpenSpec src/core/templates/skill-templates.ts:getOpsxExploreCommandTemplate (copied 2026-02-09; adapted for llman) -->

Enter explore mode for llman SDD. Think, investigate, and clarify.

**IMPORTANT: Explore mode is thinking-only (no implementation).**
- You MAY read files, search code, and run `llman sdd-legacy` commands.
- You MAY propose or draft llman SDD artifacts (proposal/specs/design/tasks) **only if the user asks**.
- You MUST NOT write application code or implement features in explore mode.

**Input**: Anything the user wants to explore (idea, problem, change id, comparison, or nothing).

## Lightweight workflow

1. Clarify the goal and constraints (ask 1–3 questions).
2. If a specific change id is relevant:
   - Run `llman sdd-legacy list --json` to confirm it exists.
   - Read artifacts under `llmanspec/changes/<id>/` (proposal/design/tasks/specs).
3. Explore options and tradeoffs (2–3 options). Use short ASCII diagrams when helpful.
4. When something crystallizes, offer to capture it (don’t auto-write):
   - Scope changes → `proposal.md`
   - Requirements → `llmanspec/changes/<id>/specs/<capability>/spec.md`
   - Design decisions → `design.md`
   - Work items → `tasks.md`
5. When ready to execute, suggest:
   - Start a change: `/llman-sdd:new` or `llman-sdd-new-change`
   - Create artifacts quickly: `/llman-sdd:ff`
   - Implement tasks: `/llman-sdd:apply`

## Guardrails
- Never implement in explore mode.
- Don’t invent evidence — ground reasoning in real repo files/commands.
- If the user asks you to implement, STOP and ask them to exit explore mode first (e.g., `/llman-sdd:new` or `/llman-sdd:ff`).

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}
