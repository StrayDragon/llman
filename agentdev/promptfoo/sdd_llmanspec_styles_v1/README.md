# sdd_llmanspec_styles_v1

Claude Code agentic eval fixture for multi-style llmanspec projects:

- `spec_style: ison`
- `spec_style: toon`
- `spec_style: yaml`

This fixture is designed to be executed via:

- `agentdev/promptfoo/run-sdd-claude-style-eval.sh`

## What v1 measures (vs v2)

v1 is a baseline that mainly exercises **CLI-driven spec generation** (`llman sdd spec add-*` / `llman sdd delta add-*`) and strict validation.

If you want a **format-sensitive** eval that forces the agent to read/edit style-specific `spec.md` files, use:

- `agentdev/promptfoo/run-sdd-claude-style-eval.sh --fixture v2`

## Hard gate

Assertions include a deterministic hard gate:

- `llman sdd validate --all --strict --no-interactive`

## Placeholders

`promptfooconfig.yaml` contains placeholders that are patched by the runner at runtime:

- `__MODEL__`, `__MAX_TURNS__`
- `__WORKDIR_{ISON|TOON|YAML}__`
- `__CONFIGDIR_{ISON|TOON|YAML}__`
