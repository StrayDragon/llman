---
name: "llman-sdd-solidify"
description: "Serialize a change's delta scenarios into executable .feature files (BDD-on only). Applies after apply, before archive. Framework-agnostic: filters by the scenario feature field and a self-reference guard, then writes Gherkin."
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD Solidify

Use this skill to generate (regenerate) the executable `.feature` files for a change, from its delta `spec.toon` scenarios. BDD-on projects only.

## Pipeline Position

```mermaid
flowchart LR
    apply["llman-sdd-apply<br/>Implement"] --> verify["llman-sdd-verify<br/>Verify"]
    verify --> solidify
    solidify["★ llman-sdd-solidify ★<br/>Solidify (you are here)"]
    solidify --> archive["llman-sdd-archive<br/>Archive"]
    archive --> commit["git commit<br/>Done"]

    style solidify fill:#fff3cd,stroke:#ffc107,stroke-width:3px
```

> 📍 You are in the solidify phase: after verify passes, before archive.
> BDD-off projects: this is a no-op (nothing to generate).

## Hard Constraints

- **BDD-on only**: if `config.yaml` has no `bdd:` block, solidify is a no-op. Stop and report.
- **Framework-agnostic**: solidify does NOT scan `tests/bdd_steps.rs` or any BDD framework's step bindings. Whether a scenario is *executable* at runtime is decided by `bdd.run_command`.
- **Don't edit `.feature` by hand**: they are generated artifacts. Edit `spec.toon` scenarios, then re-run solidify.
- **Don't ask "should I continue?"**: run to completion unless you hit an unresolvable error.

## Steps

### 1) Confirm target change
- Determine the change id (from user input or context).
- Always announce: "Solidifying change: <id>".
- `spec.toon` is the SSOT. `.feature` files are the **executable subset** of its scenarios, serialized as Gherkin.
- Scenarios whose `when` invokes `llman sdd validate|archive|solidify` are **self-referencing** and are skipped (would recurse the BDD runner).

### 2) (Optional) Dry-run preview
- `llman sdd solidify <id> --dry-run` to preview which scenarios write vs skip.
- Review the skip reasons: `feature=false` and self-referencing scenarios are expected.

### 3) Execute solidify
- `llman sdd solidify <id>`
- This writes one `.feature` per capability under `llmanspec/specs/<capability>/<capability>.feature`.

### 4) Report
- Summarize: per capability, how many scenarios written vs skipped, and the output path.
- Skipped scenarios list their reason.

> 💡 Previous phase `llman-sdd-verify` (passed) → this phase generates `.feature` → next phase `llman-sdd-archive` (archive).

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}
