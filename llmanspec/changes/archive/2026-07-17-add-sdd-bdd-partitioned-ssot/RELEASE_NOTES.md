# Release notes draft: Partitioned SSOT (BDD-on)

> Attach to the GitHub Release / changelog when shipping the llman version that includes `add-sdd-bdd-partitioned-ssot` (currently developed against `0.0.61+`).

## Highlights

- **Partitioned SSOT** for BDD-on projects: `spec.toon` = constraints; `*.feature` = executable harness (with `@req:<req_id>`).
- **`llman sdd solidify`** is a consistency gate (optional `--write-stubs`); it no longer projects toon GWT over `.feature`.
- **`llman sdd project partition-migrate`** upgrades existing dual-write trees in place (`--dry-run` supported).
- Change archives apply **`*.feature.delta.toon`** alongside toon ops.

## Breaking (BDD-on only)

| Before | After |
|---|---|
| solidify writes full `.feature` from toon | solidify = consistency only |
| Executable GWT often duplicated in toon + feature | Dual-write is a `--strict` validate error |
| Archive merges toon only | Archive also applies `feature.delta` |

BDD-off projects are unchanged.

## Upgrade

1. Upgrade llman to this release.
2. In each BDD-on repo, paste and run the agent prompt in [`UPGRADE_AGENT_PROMPT.md`](./UPGRADE_AGENT_PROMPT.md) (self-loop migrate → validate → fix).
3. Or manually:

```bash
llman sdd project partition-migrate --dry-run
llman sdd project partition-migrate
llman sdd validate --all --strict --no-check
# bind missing steps / fix @req, then:
llman sdd validate --all --strict
```

See [`MIGRATION.md`](./MIGRATION.md) for authoring rules after upgrade.
