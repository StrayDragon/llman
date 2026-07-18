Before acting, read `llmanspec/config.yaml` and follow its `context` and `rules` if present.

Common commands:
- `llman sdd context --task "<description>" --paths "<files>"` (find relevant specs). Uses the pageindex agentic tree backend (needs `LLMAN_SDD_INDEX_CHAT_MODEL`). Preset via `LLMAN_SDD_INDEX_BACKEND`.
- `llman sdd list` (list changes)
- `llman sdd list --specs` (list specs with purpose/scope metadata)
- `llman sdd show <id>` (show change/spec)
- `llman sdd validate <id>` (validate a change or spec)
- `llman sdd validate --all` (bulk validate)
- `llman sdd index rebuild` (rebuild the pageindex tree index — no model needed)
- `llman sdd index check` (check index freshness)
- `llman sdd change new <id>` (create draft `changes/<id>/proposal.md`)
- `llman sdd change attach <id> [--force]` (BDD-on: bind feature branch + base SHA)
- `llman sdd change finalize <id> [--no-check]` (BDD-on: **recommended single-commit path** — dirty tree OK; same-process checkpoint + docs-only archive; writes `checkpoint_sha = base_sha`)
- `llman sdd change checkpoint <id> [--no-check]` (BDD-on: clean tree + gates before archive; strict sha = HEAD)
- `llman sdd change diff <id> [--export-patch <path>]` (BDD-on: read-only `base...HEAD` review/export)
- `llman sdd change delta …` (BDD-off only: TOON delta authoring; rejected when BDD-on)
- `llman sdd change archive <id>` (seal a change; BDD-on: docs only after checkpoint / finalize fallback; BDD-off: merge TOON deltas)
- `llman sdd archive freeze [--before YYYY-MM-DD] [--keep-recent N] [--dry-run]` (freeze archived dirs)
- `llman sdd archive thaw [--change <id> ...] [--dest <path>]` (restore from cold-backup)
- `llman sdd graph [CHANGE] [--format mermaid] [--scope active|archived|all] [--depth N]` (generate change dependency graph)
- `llman sdd project migrate [--kind format|partitioned|legacy-bdd|auto]` (one-shot migrations)
