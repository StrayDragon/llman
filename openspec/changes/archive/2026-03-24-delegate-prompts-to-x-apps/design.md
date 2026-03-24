## Context

Current state:
- `llman prompts` owns both template storage (`$LLMAN_CONFIG_DIR/prompt/<app>/`) and tool-specific injection/writes (Cursor/Codex/Claude Code) in `src/prompt.rs`.
- `llman x <app>` already exists as the home for app-specific workflows (e.g. `llman x codex agents`), but prompt operations are not yet distributed there.

Problems:
- Blurry ownership and duplicated logic (template listing/reading + marker injection is implemented in multiple places).
- Naming confusion for Codex: “agents” can mean the `AGENTS.md` project doc or `agents/*.toml`.
- Harder to evolve integrations cleanly under `llman x <app>`.

Constraints:
- Tests/dev commands must not touch real user config; always honor `LLMAN_CONFIG_DIR`.
- Changes can be breaking, but must include migration guidance and a rollback path.

## Goals / Non-Goals

**Goals:**
- Move concrete prompt operations (template management + generation/injection) to `llman x <app> prompts`.
- Make `llman prompts` a thin interactive orchestrator that delegates to `x <app> prompts` logic.
- Provide a non-interactive escape hatch: `llman prompts --no-interactive` prints guidance to use `llman x <app> prompts ...`.
- Extract shared prompt/template utilities to avoid duplication and keep behavior consistent across integrations.

**Non-Goals:**
- Do not change the on-disk template store location (`$LLMAN_CONFIG_DIR/prompt/<app>/`) in this change.
- Do not redesign Codex/Claude/Cursor semantics beyond the CLI re-organization and naming cleanups needed to avoid ambiguity.

## Decisions

1) **Command surface split**
- `llman x cursor prompts`: manage Cursor prompt templates and generate Cursor rule files under `.cursor/rules/`.
- `llman x codex prompts`: manage Codex prompt templates and generate:
  - custom prompts under `$CODEX_HOME/prompts/` or `<repo_root>/.codex/prompts/`
  - project-doc injection into `AGENTS.md` / `AGENTS.override.md`
- `llman x claude-code prompts`: manage Claude Code prompt templates and inject into `CLAUDE.md` (global/project).
- `llman prompts`: interactive-only orchestrator. No subcommands. No app/template flags. Delegates to `llman x <app> prompts` operations.

2) **Codex target naming**
- Replace the ambiguous Codex injection target name `agents` (meaning `AGENTS.md`) with `project-doc` (or `agents-md`) at the CLI/API level.
- Keep behavior the same: marker-based, idempotent managed block injection that preserves user content outside the managed region.

3) **Shared core utilities**
- Create a shared prompt core module that owns:
  - template listing/reading/writing under `$LLMAN_CONFIG_DIR/prompt/<app>/`
  - common body composition (`## llman prompts: <name>` wrapper) where applicable
  - marker constants and managed-block update helpers
- Reuse this core from:
  - `llman x <app> prompts` commands
  - existing `llman x codex agents inject` (avoid duplicating template reading + body build)

4) **Test strategy**
- Update tests that referenced `llman prompts <subcommand>` to the new `llman x <app> prompts ...` surface.
- Add/keep a minimal test for `llman prompts --no-interactive` that validates the delegation guidance output (no interactive prompts in CI).

## Risks / Trade-offs

- [Breaking CLI scripts] → Provide clear guidance via `--no-interactive`, update docs, and document rollback as pinning/downgrading to a pre-change version.
- [Command renames cause confusion] → Use explicit naming (`project-doc`) and keep help text examples aligned.
- [Refactor touches multiple modules] → Extract shared core first, then rewire commands with minimal behavioral change; rely on existing marker-based idempotency.

## Migration Plan

1. Introduce `llman x <app> prompts` command groups and shared core utilities.
2. Rewire `llman prompts` to interactive-only orchestrator; add `--no-interactive` hint mode.
3. Update docs and i18n hints to point to the new commands.
4. Update tests to stop invoking the removed `llman prompts` subcommands and to validate delegation.

Rollback:
- Users relying on old scripting can pin/downgrade llman to a pre-change release until scripts are migrated to `llman x <app> prompts`.

## Open Questions

- Should the orchestrator support multi-app “one-shot” generation in a single flow, or sequentially delegate into each app wizard?
- Should we keep `prompt`/`rule` aliases for `llman prompts` once it becomes orchestrator-only?
