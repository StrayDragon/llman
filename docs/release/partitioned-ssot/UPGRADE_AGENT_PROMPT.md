# Agent prompt: Partitioned SSOT upgrade (self-loop)

Paste this into an agent session in a **BDD-on** llmanspec project after upgrading llman. The agent must loop until gates are green; do not stop after the first migrate.

---

You are upgrading this repo to llman SDD **Partitioned SSOT** (BDD-on).

## Goal

Make the project valid under Partitioned SSOT with no dual-write and with harness `@req` links intact. Stop only when the exit gates below are green.

## Hard rules

- Do **not** put full executable Given/When/Then back into `spec.toon` as `feature: true`.
- Do **not** run destructive git commands (`reset --hard`, `push --force`) unless I explicitly ask.
- Prefer `llman sdd` CLI output as the source of truth.
- Keep changes minimal; fix only what validate reports.

## Loop (repeat until done)

1. **Dry-run migrate**
   ```bash
   llman sdd project partition-migrate --dry-run
   ```
2. **Apply migrate** (if dry-run planned any work)
   ```bash
   llman sdd project partition-migrate
   ```
3. **Strict validate (syntax + Partitioned gates, no BDD runner)**
   ```bash
   llman sdd validate --all --strict --no-check --no-interactive
   ```
4. **Fix failures** from the report, in this order of preference:
   - Dual-write → ensure executable GWT lives only in `*.feature`; strip leftover executable rows from `spec.toon` (or re-run migrate).
   - Missing / bad `@req:X` → add or correct tags so `X` exists in `spec.toon` requirements.
   - Gherkin parse errors → fix the `.feature` file.
   - Stale / empty `valid_scope` → repair scope paths.
5. **Optional consistency**
   ```bash
   llman sdd solidify <change-id>   # if an active change exists; consistency only, not projection
   ```
6. **Full validate (runs `bdd.run_command`)**
   ```bash
   llman sdd validate --all --strict --no-interactive
   ```
   If step bindings are missing, add `#[scenario]` / step defs in the project's BDD harness (llman dogfood: `tests/bdd_steps.rs`), then re-run step 6.
7. If any command in steps 3–6 failed, go back to step 1. **Do not declare success early.**

## Done when

- `partition-migrate --dry-run` reports no remaining work (or only already-partitioned specs).
- `llman sdd validate --all --strict --no-check` exits 0.
- `llman sdd validate --all --strict` exits 0 (or document any intentional `--no-check`-only specs).
- `llman sdd list --specs --json` shows `morphology.dualWriteCount == 0` for every spec.

## Report back

Summarize: specs migrated, validate errors fixed, remaining INFO (e.g. harness without `@req`), and suggested commit message (`feat(sdd): partition-migrate to Partitioned SSOT`).
