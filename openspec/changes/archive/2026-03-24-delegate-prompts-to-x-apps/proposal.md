## Why

`llman prompts` currently mixes “template storage” and “tool-specific injection/writes” in one top-level command implementation (`src/prompt.rs`). As more integrations land under `llman x <app>`, this becomes harder to maintain and creates naming confusion (e.g., Codex “agents” can mean `AGENTS.md` or `agents/*.toml`).

We want a cleaner architecture where the real action lives under `llman x <app>` (single-app, scriptable, easy to reason about), while `llman prompts` becomes a thin interactive orchestrator for convenience.

## What Changes

- **BREAKING**: `llman prompts` becomes **interactive-only** and no longer exposes subcommands (`gen/list/upsert/rm`) or non-global options.
- Add `llman prompts --no-interactive` that **prints guidance** to use `llman x <app> prompts ...` and exits successfully.
- Move prompt template management + injection logic into:
  - `llman x cursor prompts`
  - `llman x codex prompts`
  - `llman x claude-code prompts`
- Refactor shared prompt template loading + marker-based injection utilities for reuse across `x <app> prompts` and existing `llman x codex agents inject`.

Rollback path:
- For non-interactive usage, switch to `llman x <app> prompts ...`.
- For users pinned to the old `llman prompts gen ...` scripting interface, downgrade llman to a pre-change version until scripts are migrated.

## Capabilities

### New Capabilities
- (none)

### Modified Capabilities
- `prompts-management`: redefine `llman prompts` as an interactive orchestrator and move concrete prompt operations to `llman x <app> prompts`.

## Impact

- CLI surface area: `src/cli.rs` and help text/i18n strings.
- Prompt implementation: `src/prompt.rs` (split into shared core vs per-app command wiring under `src/x/`).
- Tests: adjust prompt command tests to validate delegation/hints; move/rename tests to match new command structure.
- Docs: update examples that reference `llman prompts gen/upsert/...` to the new `llman x <app> prompts ...` form.
