---
name: "llman-sdd-ff"
description: "Fast-forward: create the planning shell then Branch binding + Specs landing in one pass. Never author under changes/<id>/specs/."
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD Fast-Forward (FF)

Run the propose-equivalent path quickly: planning shell → Branch binding → Specs landing (through `readyToImplement=true`). This is **not** the old `changes/<id>/specs/` delta model.

## Hard constraints

- **Planning shell** only under `llmanspec/changes/<id>/` (proposal/design/tasks).
- Live contracts only under bound-branch `llmanspec/specs/**` (Specs landing).
- **Do not** create `llmanspec/changes/<id>/specs/` or `*.feature.delta.toon`.
- Enter apply only when `readyToImplement=true`.

## Steps

1. Ask the user for a short description, change id (or derive), impacted capability, and confirm the final id.
2. Ensure `llman sdd init` has been run (`llmanspec/` exists).
3. If `llmanspec/changes/<id>/` exists: ask fill-missing vs new id; do not overwrite without confirmation.
4. Create the **planning shell** (OK briefly on the default branch):
   - `llman sdd change new <id>` (or hand-write) → flesh out `proposal.md`
   - `design.md` (if needed)
   - `tasks.md`
5. **Branch binding**: `llman sdd change start <id>` (clean tree on default branch) or create a branch then `change attach <id>`.
6. **Specs landing**: on the bound branch, edit live `llmanspec/specs/<capability>/<capability>.feature` and commit; or set `skip_specs_landing: true` when there is no contract edit.
7. Validate: `llman sdd validate <id> --strict --no-interactive`.
8. Confirm `readyToImplement=true` via `llman sdd show <id> --json`, then suggest `llman-sdd-apply` (do not suggest apply before ready).

{{ unit("skills/git-native-flow-brief") }}
> For command details run `llman sdd <cmd> --help`; the CLI is the command reference — skills embed no command tables (r139).
{{ unit("skills/validation-hints") }}

{{ unit("skills/ethics-governance") }}
