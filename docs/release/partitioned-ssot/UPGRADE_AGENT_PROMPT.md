# Upgrade agent prompt — unified Git-native Partitioned SSOT

You are upgrading an llman SDD project from solidify / feature_delta / dual-track lifecycle to **unified Git-native** flow.

## Goals

1. Ensure `llmanspec/config.yaml` uses `schema: spec-driven`. Add a `bdd:` block only if the project runs `.feature` harnesses via `validate --check` (runner switch — not a lifecycle fork).
2. For each **active** change under `llmanspec/changes/` (not archive):
   - If `*.feature.delta.toon` exists: materialize those ops into live `llmanspec/specs/**/*.feature`, then delete the delta files.
   - If `changes/<id>/specs/` TOON deltas exist: move useful content into live `llmanspec/specs/**`, then delete the change-scoped specs directory.
   - Clear any dual-write executable GWT (GWT belongs only in `.feature` with `@req`).
   - On the default branch with a clean tree: `llman sdd change start <change-id>` (or `change attach` on an existing feature branch).
   - For closure, prefer the single-commit path:
     ```bash
     # implement live specs + code (dirty tree is fine for finalize)
     llman sdd change finalize <change-id> --no-check
     git commit   # one commit after auto ff-merge: remaining dirty rename + impl
     ```
     `finalize` writes `checkpoint_sha = base_sha`. For strict `checkpoint_sha = HEAD`, use `change checkpoint` + `change archive` instead.
3. Confirm there is **no** `sdd solidify`, `change delta`, or `llman-sdd-sync`, and skills no longer teach them.
4. Run:
   ```bash
   llman sdd validate --all --strict --no-check
   ```
5. Do **not** rewrite archived history under `llmanspec/changes/archive/` except to leave it frozen/readable.

## Non-goals

- Do not invent a second delta store or restore solidify / `change delta`.
- Do not require remote push unless `LLMAN_SDD_REQUIRE_UPSTREAM=1`.
- Do not call `project migrate --kind partitioned` — that kind is removed; clean leftovers manually.

## Done when

- `validate --all --strict --no-check` passes
- No active `*.feature.delta.toon` or `changes/<id>/specs/`
- Active changes have Git binding in proposal frontmatter (or are archived)
- Skills/docs describe a single Git-native lifecycle with `change start` / `finalize`
