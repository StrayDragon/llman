# Design: add-sdd-global-req-id-uniqueness

## Goals

1. Main-library `req_id` **globally unique**, kept as a **short alias** (`r12`, custom tags)
2. Validate enforces uniqueness (strict → ERROR)
3. CLI allocates and **resolves** ids so agents need not encode capability into the id
4. One-shot repo dedupe remap (short ids only)
5. Dogfood `feature_delta`

## Non-goals

- Embedding capability into `req_id` (rejected: `{capability}--{old}`)
- Rewriting `llmanspec/changes/archive/**`
- Changing scenario `id` rules (still per-spec `(req_id, scenario_id)`)

## Model

```
req_id  = short opaque alias (global unique key)
capability / title / statement / @req features  = resolved via CLI / index
```

Agents: `next-req-id` → write → `resolve-req` / `show` / `context` when they need ownership or text.

## Migration (batch dedupe)

For each colliding `req_id` appearing in ≥2 capabilities:

1. Keep one occurrence (stable preference: lexicographically first capability path, or first-seen — pick one and document)
2. Remap other occurrences to fresh short ids from the allocator
3. Update that capability's toon `requirements` / `scenarios[].req_id` and its `.feature` `@req:` tags
4. Ids already unique globally: **unchanged**

Idempotent: second run finds no collisions.

This change's own delta ids (`r9`, `r6`, `r62`…) stay short; after main-library remap, ensure they still don't collide (allocator / manual check in apply).

## Validate algorithm

Runs on common validate entrypoints that load the main library (`--all`, single spec, change paths that consult main specs) — **immediately**, not deferred:

```
seen: Map<req_id, capability>
for each main capability requirements:
  if req_id in seen and seen[req_id] != this capability:
    ERROR (default and --strict) with:
      - conflicting req_id
      - both (all) capability names
      - fix hint: `llman sdd spec next-req-id` and/or `resolve-req <id>`
  else seen[req_id] = capability
```

Do **not** soft-warn-only on the default path: duplicate ids are debt; fail closed.

Per-doc duplicates unchanged. `@req:X` still requires `X` in **this** capability's requirements.

## CLI

### `llman sdd spec next-req-id [--json]`

1. Scan all main-library req_ids
2. Default: smallest unused positive `rN` among ids matching `^r(\d+)$`
3. `--json`: `{ "reqId": "r85" }`

No `--capability` required for allocation (capability is chosen when writing the spec row).

### `llman sdd spec add-req`

If `req_id` already used in any main capability → non-zero exit; stderr names conflicting capability.

### `llman sdd spec resolve-req <req_id> [--json]`

Lookup in main library:

```json
{
  "reqId": "r12",
  "capability": "errors-exit",
  "title": "...",
  "statement": "...",
  "harness": [{ "feature": "...", "scenario": "..." }]
}
```

Missing id → non-zero. This is the primary **mapping/display** surface for agents.

### `llman sdd project dedupe-req-ids [--dry-run]`

Batch remap colliding short aliases: keep lex-first capability; others get fresh `rN`. `@req` rewrite is id-boundary safe (`r1` does not touch `r10`).

## feature_delta (dogfood)

| File | Target | Purpose |
|---|---|---|
| `sdd-bdd-mode-compat.feature.delta.toon` | `global-req-id.feature` | collision strict/warn |
| `sdd-workflow.feature.delta.toon` | `global-req-id-authoring.feature` | next-req-id / add-req / resolve-req |

Constraint notes: toon `op_scenarios` with `feature:false`.

## Rollback

Git revert. Remap only rewrites colliding short ids; no prefix strip needed.
