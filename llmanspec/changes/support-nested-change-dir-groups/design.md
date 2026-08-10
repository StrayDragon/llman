# Design: nested change dir discovery

## Context

Active changes are discovered only as direct children of `llmanspec/changes/` with `proposal.md`. Users group candidates under intermediate folders; nested changes are invisible to list/show/status/validate/graph and all `changes.join(id)` call sites.

## Decisions

### D1 — Leaf id + recursive discovery (not path-shaped ids)

**Choice**: Change id remains the leaf directory name; groups are organizational only.

**Rejected**: Path-shaped ids (`some_a/c0`) — conflicts with `validate_sdd_id` (no `/`) and prefix-match (`r112`).

**Consequence**: Leaf ids MUST be unique across the whole active tree; discovery fails hard on duplicates.

### D2 — Discovery SSOT + resolve map

**Choice**: `list_changes` / `discover_changes` recursively finds dirs with `proposal.md` (skip `archive/`, dot dirs, symlinks). Expose `resolve_change_dir(root, id)` (and path relative to `changes/`). All command surfaces MUST use it; ban ad-hoc `changes.join(id)` for active changes.

**Depth**: Default `8` (depth 1 = direct child of `changes/`). Effective depth from `llman sdd --max-scan-depth <N>` only (thread via process-local / clap args into discovery). No `config.yaml` field.

### D3 — Conflict = discover-time ERROR

**Choice**: Duplicate leaf ids → `Err` with all conflicting relative paths listed (agent-friendly). No silent drop.

### D4 — list/show JSON `path`

**Choice**: Add `"path"` relative to `llmanspec/changes/` (e.g. `some_a/c0`). Flat layout: `path` equals id (or identical string). Human `show` prints a `path:` line. `id` / `name` fields stay leaf ids.

### D5 — graph: remove partial-node heuristic

**Choice**: Delete “direct child without `proposal.md` → partial node”. Group folders must not appear as nodes. Edges stay leaf-id based.

### D6 — archive flattens; no former_path

**Choice**: Archive always to `changes/archive/<date>-<leaf-id>/`. No group path retention (forward-only active→archive).

### D7 — `change new` unchanged

**Choice**: Still creates `changes/<id>/` only. Users mkdir groups manually. No `--group`.

## Risks

| Risk | Mitigation |
|------|------------|
| Missed `changes.join(id)` call site | Grep + shared helper; unit/integration cover nested show/start |
| Depth flag only on one subcommand | Top-level `SddArgs` so all subcommands share |
| Prefix match across nested ids | Still leaf-id list; conflict paths in multi-match / not-found hints |
| Deep accidental trees | Default depth 8 + no symlink follow |

## Spec landing plan

- Extend `sdd-workflow` with new req(s) `@req` features for A–F (CLI-driven where possible; discovery conflict / depth may be unit + CLI).
- Touch `cli` only if `--max-scan-depth` is specified there; prefer documenting under `sdd-workflow` + clap on `SddArgs`.
