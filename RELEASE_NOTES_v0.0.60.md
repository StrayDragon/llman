# llman v0.0.60 — Release Notes

**Previous**: [`v0.0.59`](../../releases/tag/v0.0.59) (2026-07-14)
**Commits in this release**: 34 · **Files changed**: 291 (+9305 / −4616)

---

## ⭐ Headline: BDD-on mode (feature-as-spec)

This release introduces **BDD-on mode**: spec behavior can now live in executable
`.feature` files (Gherkin) that double as both a human/AI-readable contract *and*
a runnable test. The entire llman spec library (30 specs) has been migrated to
this model, and llman itself now dogfoods it (`cargo test --features bdd`).

> **Already on BDD-off? You don't have to change anything.** BDD-on is opt-in
> per project. If your `config.yaml` has no `bdd:` block, every command behaves
> exactly as before — `.feature` files (if any) are silently ignored. See
> *Compatibility* below.

### How to enable BDD-on in your project

Add a `bdd:` block to `llmanspec/config.yaml`:

```yaml
schema: spec-driven
locale: zh-Hans

bdd:
  run_command: "cargo test --features bdd"                      # rstest-bdd
  # run_command: "pytest {feature_dir} -k {feature_name} -v"    # pytest-bdd
  # run_command: "npx cucumber-js {feature_path}"               # cucumber-js
```

The `sdd solidify` command then serializes a change's delta scenarios into
`.feature` files; `sdd validate --check` runs them for real.

---

## ✨ New features (user-visible)

### `sdd solidify <change>` — generate executable `.feature` files
- Serializes a change's delta scenarios (from `spec.toon`) into Gherkin `.feature`
  files under `llmanspec/specs/<capability>/`.
- Framework-agnostic: filters by the scenario `feature` field + a self-reference
  guard. Whether a scenario *runs* is decided by `bdd.run_command`.
- BDD-off projects: no-op (prints a clear message, does nothing).
- **BDD-mode-aware**: if BDD is off but stale `.feature` files exist, reports a
  residual warning (lists them, shows how to re-enable) — never deletes them.

### `sdd validate --check` / `--no-check` — two-tier validation
- **Fast mode** (default): parses `.feature` Gherkin structure (syntax only).
  Always runs; no test runner invoked.
- **Full mode** (`--check`, or auto when `bdd.run_command` is configured): runs
  the BDD runner (`cargo test --features bdd`, etc.) for real execution.
- `--no-check` skips the runner even when BDD is configured.
- BDD-off + `--check`: downgraded to an INFO message (not an error).

### `sdd status` — redesigned as compact agent-facing TOON
- Default output is now pure TOON format (`kind: llman.sdd.status` + `counts`),
  optimized for agent consumption instead of human prose.
- `--json` still available.

### `llmanspec` binary — thin wrapper for `llman sdd`
- A standalone `llmanspec` binary that delegates to `llman sdd`, for projects
  that want a spec-only entry point.

### BDD-aware context index (`.feature` content retrieval)
- `sdd index rebuild` now embeds `.feature`-derived scenarios (req_id empty,
  "spec-level") into `tree.json`. The retrieval agent can now see the real
  Given/When/Then behavior details, not just bare MUST/SHALL statements.
- `sdd context` surfaces spec-level scenarios separately so they don't vanish
  under the requirement filter.

---

## 🔧 Improvements

- **`spec.toon` is now the SSOT.** Config `context`/`rules` fields removed;
  project rules now live directly in `AGENTS.md` (less indirection). `spec.toon`
  carries `valid_scope` + `requirements` + `scenarios` inline.
- **Solidify skill standardized** with zh-Hans i18n; all SDD skills ship in both
  `en` and `zh-Hans`.
- **propose skill** now checks BDD mode up front (step 4a) and, for
  executable-behavior changes, asks whether to enable BDD-on — never silently.
- **Spec compaction**: dropped dead legacy ISON/multi-style contracts across the
  spec library; Chinese descriptions enforced where the project locale is zh-Hans.

---

## 🐛 Fixes

- **`fix(sdd)`**: staleness base-ref resolution was broken by a `--` separator
  in the git command — staleness checks now resolve the base ref correctly.
- **`fix(doc)`**: broken intra-doc link `[ScenarioNode]` in `solidify.rs`
  (caught by `just check-all`'s doc-check gate).

---

## 📦 Compatibility

**Non-BDD projects are unaffected.** Verified by a dedicated test suite
(`tests/sdd_bdd_compat_tests.rs` + `llmanspec/specs/sdd-bdd-mode-compat/*.feature`,
494 default tests + 11 BDD scenarios green):

| Command | BDD-off (no `bdd:` block) | BDD-on |
|---|---|---|
| `validate` | ignores `.feature` files silently | auto-runs runner (unless `--no-check`) |
| `solidify` | no-op, clear message | generates `.feature` |
| `index rebuild` | no spec-level scenarios | embeds `.feature` scenarios |
| `context` (retrieval) | works as before | richer (includes `.feature` G/W/T) |

- **`tree.json` backward compatibility**: old indexes (pre-`.feature` scenarios
  field) still load via `#[serde(default)]`; they gain scenarios on next rebuild.
- **Disabling BDD** does **not** delete existing `.feature` files — `validate`/
  `index` ignore them. `solidify` warns about residuals but won't remove them.

### Migration
- Existing projects: no action required. To opt into BDD-on, add the `bdd:` block
  (shown above) and run `llman sdd solidify <change>` for changes with executable
  scenarios.
- `llman sdd project solidify-migrate` one-shot migrates legacy BDD-on specs
  (minimal `spec.toon` + `.feature`) to the unified full structure. Idempotent.

---

## 🧪 For contributors / agents

- **New spec**: `llmanspec/specs/sdd-bdd-mode-compat/` documents the BDD on/off
  behavior contract as executable `.feature` scenarios.
- **AGENTS.md** gains: BDD-on conventions, the rstest-bdd placeholder quote trap,
  BDD-mode-compat test maintenance rules, and a "how to enable/disable BDD" guide.
- Run the full pre-release gate with `just check-all` (fmt + clippy + tests +
  doc-check + release build + sdd-templates + schemas). BDD scenarios:
  `cargo test --features bdd`.

---

## Upgrade

```bash
cargo install llman --locked
# or from source:
git checkout v0.0.60 && cargo install --path .
```

Then in any SDD project: `llman sdd init --update` to refresh installed skills
(includes the new BDD-mode-aware solidify/propose).
