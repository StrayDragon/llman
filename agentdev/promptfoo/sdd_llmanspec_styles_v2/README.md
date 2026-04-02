# sdd_llmanspec_styles_v2

Claude Code agentic eval fixture for multi-style llmanspec projects (format-sensitive):

- `spec_style: ison`
- `spec_style: toon`
- `spec_style: yaml`

This fixture is designed to be executed via:

- `agentdev/promptfoo/run-sdd-claude-style-eval.sh --fixture v2`

## What v2 measures (vs v1)

v1 is a baseline that mainly exercises **CLI-driven spec generation** (`llman sdd spec add-*` / `llman sdd delta add-*`) and strict validation.

v2 is **format-sensitive**:

- The runner pre-seeds each workspace with semantically equivalent baseline content.
- The task forces the agent to **read** and **edit** the style-specific `spec.md` files directly.
- Assertions verify the agent read main specs and performed deterministic marker edits.

This makes the (ison/toon/yaml) format differences materially affect the agent’s context and actions, so tokens/turns/cost are more attributable to the spec format itself.

## Hard gate

Assertions include a deterministic hard gate:

- `llman sdd validate --all --strict --no-interactive`

## Placeholders

`promptfooconfig.yaml` contains placeholders that are patched by the runner at runtime:

- `__MODEL__`, `__MAX_TURNS__`
- `__WORKDIR_{ISON|TOON|YAML}__`
- `__CONFIGDIR_{ISON|TOON|YAML}__`
- `__PATH_{ISON|TOON|YAML}__`
