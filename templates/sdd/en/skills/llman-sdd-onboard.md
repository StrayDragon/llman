---
name: "llman-sdd-onboard"
description: "Onboard to the llman SDD workflow in a repository."
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD Onboard

Use this skill to onboard to llman SDD in a repository.

## Steps
1. Read `llmanspec/config.yaml` for project context, conventions, and rules.
2. Use `llman sdd list --specs --json` to see all specs at a glance.
   - Or use `llman sdd context --task "<task description>" --paths "<files>"` to find task-relevant specs.
   - If context returns `quality: "unavailable"`, run `llman sdd index rebuild` first (default backend is `pageindex`; it needs `LLMAN_SDD_INDEX_CHAT_MODEL` for retrieval but not for rebuilding).
3. Read only the `direct` spec files from context output.
4. Assess change scale (see triage rules): behavioural contract change → full SDD; implementation change → quick path.
5. Advance by path:
   - **Full path**: Designed planning shell → Branch binding → Specs landing (or `skip_specs_landing`) → `readyToImplement=true` → apply → verify → archive (skill navigation: propose → apply → verify → archive).
   - **Quick path**: no MUST/SHALL change; edit code and commit (live specs only on a bound branch — see `llman-sdd-quick`).
6. Use `llman sdd graph` to visualize change dependencies.

{{ sdd_command_reference }}

## Notes
- `llmanspec/config.yaml` holds project context, rules, locale, and skills paths.
- Locale affects templates/skills only; CLI stays English.
- Refresh skills with `llman sdd init --update`.

{{ unit("skills/validation-hints") }}

{{ unit("skills/ethics-governance") }}
