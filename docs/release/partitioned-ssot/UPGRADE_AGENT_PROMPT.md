# Upgrade agent prompt — Git-native Partitioned SSOT

You are upgrading an llman SDD project from solidify / feature_delta to **Git-native BDD-on**.

## Goals

1. Ensure `llmanspec/config.yaml` has a `bdd:` block if the project uses `.feature` harnesses.
2. Run `llman sdd project migrate --kind partitioned` to clear remaining toon↔feature dual-writes.
3. For each **active** change under `llmanspec/changes/` (not archive):
   - If `*.feature.delta.toon` exists: materialize those ops into live `llmanspec/specs/**/*.feature`, then delete the delta files.
   - Prefer moving leftover change TOON deltas into live `spec.toon` on a feature branch (BDD-on archive no longer merges them).
   - Create/switch to a non-default feature branch, then:
     ```bash
     llman sdd change attach <change-id>
     # commit binding
     llman sdd change checkpoint <change-id> --no-check
     ```
4. Confirm there is **no** `sdd solidify` command and skills no longer mention it.
5. Run:
   ```bash
   llman sdd validate --all --strict --no-check
   ```
6. Do **not** rewrite archived history under `llmanspec/changes/archive/` except to leave it frozen/readable.

## Non-goals

- Do not invent a second delta store or restore solidify.
- Do not require remote push unless `LLMAN_SDD_REQUIRE_UPSTREAM=1`.
- BDD-off projects: skip attach/checkpoint; keep TOON delta archive.

## Done when

- `validate --all --strict --no-check` passes
- No active `*.feature.delta.toon`
- Active BDD-on changes have Git binding in proposal frontmatter (or are archived)
- Skills/templates describe attach → checkpoint → docs-only archive → Git merge
