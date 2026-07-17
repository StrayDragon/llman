---
name: "llman-sdd-solidify"
description: "Partitioned SSOT: consistency gate for harness vs constraints (optional --write-stubs). After apply, before archive."
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD Solidify (Partitioned)

Under BDD-on, `.feature` is the executable harness authority; `spec.toon` is the constraints authority. Solidify does **not** project toon `op_scenarios` GWT over `.feature`.

## Pipeline

`apply → verify → solidify → archive`

## Hard constraints

- BDD-off: no-op with not-configured message.
- BDD-on: check `@req` links, dual-write, non-executable id intrusion; non-zero on failure.
- `--write-stubs`: only `feature_delta` **add** ops for missing scenario ids; **must not** overwrite existing GWT.
- Executable scenario changes belong in `*.feature.delta.toon`, not dual-written into toon scenarios.

## Commands

```bash
llman sdd solidify <change-id> [--dry-run] [--write-stubs]
```

Success stdout includes `consistency ok`.

## Downstream migration

```bash
llman sdd project partition-migrate [--dry-run]
```

{{ unit("skills/sdd-commands") }}
{{ unit("skills/structured-protocol") }}
