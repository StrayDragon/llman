# llman sdd project migrate — collaboration notes

## Command intent

- `llman sdd project migrate --kind toon2features`: legacy `spec.toon` → single-track `.feature` (one-shot, idempotent).
- `llman sdd project migrate --kind specs-flatten`: pure single-file directories `specs/<cap>/<cap>.feature` → flat `specs/<cap>.feature` (git mv preserves history).

## What the agent does

- First confirm migration is actually needed (no legacy / single-file dirs → no-op).
- Run `--dry-run` first and read the precheck report; resolve `conflict` / `misnamed` entries manually — never force.
- After migrating, run `llman sdd validate --specs --strict --no-interactive` and the project BDD suite.

## What the human does

- Review the `scope_rewritten` report and the git diff (moves keep history).
- Optionally point `# scope:` at the real source directory the spec governs.

## Pitfalls

- Dirs with multiple `.feature` files, foreign-named files, or auxiliary entries are NOT flattened (reported only).
- Name conflicts must be resolved manually (both files are kept).
- Self-referential `# scope:` entries are auto-rewritten to `specs/<cap>.feature`.

## Next steps

- `llman sdd validate --specs --strict --no-interactive`
- project BDD run (e.g. `cargo test --features bdd`)
