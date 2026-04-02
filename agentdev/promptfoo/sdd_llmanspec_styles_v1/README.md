# sdd_llmanspec_styles_v1

Claude Code agentic eval fixture for multi-style llmanspec projects:

- `spec_style: ison`
- `spec_style: toon`
- `spec_style: yaml`

This fixture is designed to be executed via:

- `agentdev/promptfoo/run-sdd-claude-style-eval.sh`

## Hard gate

Assertions include a deterministic hard gate:

- `llman sdd validate --all --strict --no-interactive`

## Placeholders

`promptfooconfig.yaml` contains placeholders that are patched by the runner at runtime:

- `__MODEL__`, `__MAX_TURNS__`
- `__WORKDIR_{ISON|TOON|YAML}__`
- `__CONFIGDIR_{ISON|TOON|YAML}__`
