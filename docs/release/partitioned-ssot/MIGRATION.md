# Downstream migration: Partitioned SSOT (BDD-on)

Breaking change in llman SDD when `llmanspec/config.yaml` contains `bdd:`.

## What changed

| Before | After |
|---|---|
| `spec.toon` scenarios (feature:true) projected to `.feature` by solidify | **Partitioned**: `.feature` = harness SSOT; `spec.toon` = constraints + non-executable scenarios |
| GWT often duplicated | Dual-write is a validate error under `--strict` |
| solidify writes full `.feature` from toon | solidify = consistency gate (+ optional `--write-stubs`) |
| Archive merges toon only | Archive also applies `*.feature.delta.toon` |

## Upgrade steps

**Recommended:** paste [`UPGRADE_AGENT_PROMPT.md`](./UPGRADE_AGENT_PROMPT.md) into an agent and let it self-loop migrate → validate → fix until green.

**Manual:**

```bash
# 1. Install/upgrade llman to the release that includes Partitioned SSOT
# 2. In each BDD-on project:
llman sdd project partition-migrate --dry-run
llman sdd project partition-migrate
llman sdd validate --all --strict --no-check
# 3. Fix any remaining @req / step bindings, then:
llman sdd validate --all --strict   # runs bdd.run_command
```

Release blurb draft: [`RELEASE_NOTES.md`](./RELEASE_NOTES.md).

## Authoring after upgrade

- Constraints / architecture: edit `spec.toon` requirements (+ `feature:false` scenarios if needed)
- Executable behavior: edit `.feature` (with `@req:<req_id>`) or change `*.feature.delta.toon`
- Do **not** put full executable GWT back into toon `feature:true` rows

## Rollback

Restore pre-upgrade `spec.toon` + `.feature` from git; pin llman to previous version. There is no automatic reverse-migrate.
