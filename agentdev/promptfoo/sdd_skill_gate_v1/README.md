# sdd_skill_gate_v1

Skill-gate evaluation fixture: measures **SDD skill template quality** by combining
two capabilities that previously lived in separate suites:

1. **Skill-in-prompt** (from `sdd_apply_v1`): render the target SKILL.md into the
   agent's prompt as the system instruction.
2. **Sandbox + hard gate** (from `sdd_llmanspec_styles_v1`): drive a real agentic
   task in an isolated llmanspec workspace, with `llman sdd validate --all --strict`
   as the deterministic pass/fail gate.

## What it measures (vs the other two suites)

| aspect | sdd_apply_v1 | sdd_llmanspec_styles_v1 | **sdd_skill_gate_v1** |
|---|---|---|---|
| skill-in-prompt | ✅ | ❌ | ✅ |
| sandbox + hard gate | ❌ (chat only) | ✅ | ✅ |
| A/B baseline | ⚠️ degenerate | ❌ | ✅ (`--baseline-skill`) |
| dimension | — | spec format (ison/toon/yaml) | skill template version (baseline/candidate) |

## Execution

Driven by `agentdev/promptfoo/run-sdd-skill-gate-eval.sh` (or the
`scripts/sdd-skill-gate-eval.sh` wrapper). Examples:

```bash
# Dry-run (no API calls): generate sandboxes + prompts + patched config
bash scripts/sdd-skill-gate-eval.sh --no-run

# Single-version eval (candidate = current workspace, no baseline comparison)
bash scripts/sdd-skill-gate-eval.sh --cc-account glm-lite-150

# A/B: compare previous template snapshot against current workspace
git show HEAD~1:templates/sdd/zh-Hans/skills/llman-sdd-apply.md > /tmp/apply-prev.md
bash scripts/sdd-skill-gate-eval.sh --baseline-skill /tmp/apply-prev.md --runs 2 --cc-account glm-lite-150
```

## Placeholders (patched by runner at runtime)

`promptfooconfig.yaml` contains placeholders the runner fills in:

- `__MODEL__`, `__MAX_TURNS__`
- `__WORKDIR_BASELINE__` / `__WORKDIR_CANDIDATE__`
- `__CONFIGDIR_BASELINE__` / `__CONFIGDIR_CANDIDATE__`
- `__PATH_BASELINE__` / `__PATH_CANDIDATE__`

## MVP scoring (P1)

- **Hard gate** (deterministic, pass/fail): `llman sdd validate --all --strict` exit code.
- **Cost** (promptfoo native): tokens (prompt/completion/total), turns, cost.
- **No LLM-rubric** by default (`--judge off`); can be enabled via `--judge codex|claude` (P3 territory).

## P1 scope boundary

P1 covers:
- The `llman-sdd-apply` skill only.
- A single shared `agent_task.md` prompt (candidate skill text).
- Baseline-vs-candidate via `--baseline-skill <path>` snapshot.

P1 does **not** cover (deferred):
- draft / propose / quick / verify / archive skills (P2).
- Persisted baseline storage across runs (P3).
- LLM-rubric + golden reference anchors (P3).
- Per-variant distinct prompts within one eval run (P1 uses candidate text for both
  providers; true baseline text comparison requires separate runs because promptfoo
  does not natively support per-provider prompt files in one config).
