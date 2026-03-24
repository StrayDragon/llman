## 1. Shared prompt core

- [x] 1.1 Extract shared template store helpers for `$LLMAN_CONFIG_DIR/prompt/<app>/`
- [x] 1.2 Extract shared marker-based managed-block injection helpers
- [x] 1.3 Refactor `llman x codex agents inject` to reuse the shared core (no behavior change)

## 2. `llman x <app> prompts` command groups

- [x] 2.1 Add `llman x cursor prompts` with `gen/list/upsert/rm` (plus wizard default)
- [x] 2.2 Add `llman x codex prompts` with `gen/list/upsert/rm` (targets: `prompts|project-doc`)
- [x] 2.3 Add `llman x claude-code prompts` with `gen/list/upsert/rm`

## 3. `llman prompts` orchestrator

- [x] 3.1 Replace top-level `llman prompts` with interactive-only orchestrator (no subcommands)
- [x] 3.2 Implement `llman prompts --no-interactive` to print migration guidance and exit 0

## 4. Docs + i18n

- [x] 4.1 Update i18n hints/help strings that reference old `llman prompts <subcommand>` usage
- [x] 4.2 Update docs examples to use `llman x <app> prompts ...` where applicable

## 5. Tests + verification

- [x] 5.1 Update/remove tests that invoke removed `llman prompts <subcommand>` surface
- [x] 5.2 Add a non-interactive integration test asserting `llman prompts --no-interactive` prints delegation guidance
- [x] 5.3 Run `just test` (or `cargo +nightly test --all`) and fix prompt-related failures only
