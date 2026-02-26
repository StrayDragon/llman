---
name: "llman-sdd-explore"
description: "Enter explore mode for llman SDD (thinking only; no implementation)."
metadata:
  llman-template-version: 1
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
1. Check context: `llman sdd list --json`
2. If a change id is relevant, read its artifacts under `llmanspec/changes/<id>/`.
3. Ask 1-3 clarifying questions, then explore options and tradeoffs.
4. When something crystallizes, offer to capture it (don’t auto-write):
   - Scope changes → `proposal.md`
   - Requirements → `llmanspec/changes/<id>/specs/<capability>/spec.md`
   - Design decisions → `design.md`
   - Work items → `tasks.md`

## Exiting explore mode
When the user is ready to implement, suggest:
- `/llman-sdd:new` or `llman-sdd-new-change` (start a change)
- `/llman-sdd:ff` or `llman-sdd-ff` (create all artifacts quickly)
- `llman-sdd-apply` (implement tasks)

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}
{{ unit("skills/future-planning") }}
