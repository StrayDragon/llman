---
name: "llman-sdd-draft"
description: "Quickly capture a change idea as a draft proposal (proposal.md only, via `change new --from`). No tasks/design/specs/attach. Use to jot down ideas or future requirements; promote to full propose when ready."
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD Draft

Capture a change idea as a **draft proposal** (a `proposal.md` skeleton only). This is the lightweight entry point for "just record this idea / future need" — no triage, no tasks, no live specs, no attach. Promote to a formal change with `llman-sdd-propose` when the idea is ready to act on.

## Pipeline Position

```mermaid
flowchart LR
    draft["★ llman-sdd-draft ★<br/>Draft (you are here)"] -.->|"promote"| propose["llman-sdd-propose<br/>Propose"]
    propose --> apply["llman-sdd-apply<br/>Implement"]
    apply --> verify["llman-sdd-verify<br/>Verify"]
    verify --> archive["llman-sdd-archive<br/>Archive"]

    style draft fill:#fff3cd,stroke:#ffc107,stroke-width:3px
```

> 📍 You are at the draft stage → next: flesh out `proposal.md`, then run `llman-sdd-propose` to formalize
> 📎 This skill creates a **draft** change (proposal.md only). For full proposals (tasks + specs + attach), use `llman-sdd-propose`.

## Hard Constraints

- **MUST NOT ask the user for a change id**: derive it from the description via `change new --from` and announce it.
- **MUST NOT create tasks/design/specs/attach**: this skill creates only the `proposal.md` draft shell. Full planning artifacts belong to `llman-sdd-propose`.
- **MUST NOT run triage or assess change scale**: that is propose's job. If the user wants to start implementing, suggest `llman-sdd-propose`.
- **Scope boundary**: if the description clearly involves MUST/SHALL behavioral contract changes or multi-file impact, suggest `llman-sdd-propose` instead of stopping at a draft — but still create the draft shell first so the idea isn't lost.

## Steps

### 0) Preflight
- Read `llmanspec/config.yaml` for project context, rules, locale.
- `llmanspec/` must exist; if missing, tell the user to run `llman sdd init`, then STOP.

### 1) Capture the description
- Take the user's description as-is (e.g. "draft: add a export-to-json command", "note down: we should support worktrees for sdd changes").
- **MUST NOT ask for a change id.** Derive it from the description.

### 2) Create the draft shell
```bash
llman sdd change new --from "<user description>"
```
- The CLI generates a legal kebab-case id (sanitized + validated), creates `llmanspec/changes/<derived id>/proposal.md` (a skeleton with `## Why` / `## What Changes` TODO sections), and prints the final id + path.
- If the derived id collides with an existing change, the CLI fails non-zero; suggest rephrasing the description or using `--force` to overwrite (rare for drafts).

### 3) Announce and hand off
- **MUST tell the user the derived id** (e.g. "Created draft change `<id>` at `llmanspec/changes/<id>/proposal.md`").
- Suggest next steps:
  - Flesh out `proposal.md` (Why / What Changes / Capabilities / Impact) now or later.
  - When ready to act on it, run `llman-sdd-propose` to formalize (triage + tasks + live specs + `change start`).

> 💡 Draft captured → next: edit `proposal.md`, then `llman-sdd-propose` to formalize.

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}
