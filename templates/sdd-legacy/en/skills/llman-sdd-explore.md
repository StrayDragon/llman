---
name: "llman-sdd-explore"
description: "Enter explore mode for llman SDD (thinking only; no implementation)."
metadata:
  llman-template-version: 2
---

# LLMAN SDD Explore

Use this skill when the user wants to think through ideas, investigate problems, or clarify requirements **before** starting implementation.

**IMPORTANT: Explore mode is for thinking, not implementing.**
- You MAY read files, search code, and investigate the codebase.
- You MAY create or update llman SDD artifacts (proposal/specs/design/tasks) if the user asks.
- You MUST NOT write application code or implement features in explore mode.

## Stance
- Curious, not prescriptive
- Grounded in the actual codebase
- Visual when helpful (ASCII diagrams)
- Willing to hold multiple options and tradeoffs

## Suggested moves
1. Clarify the goal and constraints (ask 1–3 questions).
2. Check context: `llman sdd-legacy list --json`
3. If a change id is relevant, read its artifacts under `llmanspec/changes/<id>/`.
4. Explore options and tradeoffs (2–3 options).
5. When something crystallizes, offer to capture it (don’t auto-write):
   - Scope changes → `proposal.md`
   - Requirements → `llmanspec/changes/<id>/specs/<capability>/spec.md`
   - Design decisions → `design.md`
   - Work items → `tasks.md`

## Exiting explore mode
When the user is ready to implement, suggest:
- `/llman-sdd:new` or `llman-sdd-new-change` (start a change)
- `/llman-sdd:ff` or `llman-sdd-ff` (create all artifacts quickly)
- `llman-sdd-apply` (implement tasks)
If the user asks you to implement while in explore mode, STOP and remind them to exit explore mode first.

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}
