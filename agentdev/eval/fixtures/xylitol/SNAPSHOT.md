# Fixture: `xylitol`

Frozen SDD spec corpus used as the **stable eval benchmark**. Independent of
llman's own `llmanspec/`, so llman can iterate freely without benchmark drift.

## Provenance

| field | value |
|-------|-------|
| source project | `xylitol` (an agent-runtime codebase, ~43 specs, rich change history) |
| snapshot date | 2026-06-29 |
| source HEAD | `82f8c197e0e0d3bd4eae1ef8f50c0932c6ec8bba` |
| how taken | `cp -r <src>/llmanspec/{specs,changes,config.yaml}` (excludes `.context/`) |

## Why this corpus

Selected over 8 candidate `*/llmanspec` projects for the eval benchmark because:

- **100 eval cases** — each archived change with a `specs/` delta is a labelled
  example (task ← proposal title, gold ← delta dir names). Most candidates had 0.
- **Wide gold coverage** — gold spans 46 distinct specs (≈ all 43), vs llman's
  own corpus where gold concentrates on `sdd-workflow`.
- **Multi-spec cases** — e.g. `c260-refactor-domain-architecture` touches 11
  specs at once. llman's corpus has only single-spec cases; these stress
  recall + precision under real cross-cutting changes.
- **Format-compatible** — specs are `spec.toon` (43/47), the format llman's
  `index rebuild` consumes. (4 legacy `spec.md` are ignored by indexing, harmless.)
- **Bilingual tasks** — mix of Chinese and English proposal titles.

## What is frozen vs rebuilt

| path | frozen? | why |
|------|---------|-----|
| `specs/` | ✅ | the retrieval corpus — must stay constant so the benchmark is reproducible |
| `changes/archive/` | ✅ | the labelled examples (task + gold); the ground truth |
| `config.yaml` | ✅ | spec_style etc. |
| `.context/` (index) | ❌ rebuilt each run | the index is produced by the **current** llman binary — that's the thing under test |

So updating llman (tree builder, system prompt, spec parser) never changes the
benchmark; it only changes what the variants produce against the same frozen corpus.

## Updating the snapshot

Deliberately manual — a benchmark must not drift silently. To refresh (e.g.
pull in newer xylitol history), bump the source HEAD above and re-copy:

```bash
SRC=../../../xylitol/llmanspec   # adjust to current path
rm -rf fixtures/xylitol/llmanspec
mkdir -p fixtures/xylitol/llmanspec
cp -r "$SRC/specs" "$SRC/changes" fixtures/xylitol/llmanspec/
cp "$SRC/config.yaml" fixtures/xylitol/llmanspec/config.yaml
find fixtures/xylitol -name '.context' -prune -exec rm -rf {} +
```

Then regenerate `cases.json` and re-baseline.
