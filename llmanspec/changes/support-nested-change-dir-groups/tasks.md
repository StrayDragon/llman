# Tasks

## Seams (confirmed)

- A `llman sdd list` / `--json` — nested visible; JSON `path`
- B `llman sdd show <leaf-id> [--json]` — resolve nested; `path`
- C `llman sdd graph` — no group/partial nodes
- D discovery conflict — non-zero + relative paths
- E `llman sdd --max-scan-depth <N>` — default 8; shared across commands
- F `change start` / `archive` smoke — resolve nested; flat archive

## Vertical slices

- [ ] T1 Discovery SSOT: recursive `proposal.md` walk (skip `archive`/dot/symlink), default depth 8, duplicate leaf id → Err with relative paths; `resolve_change_dir` + relative `path`; unit tests for A/D/E core. Seam D/E.
- [ ] T2 Wire `--max-scan-depth` on top-level `llman sdd` (`SddArgs`); pass effective depth into all discovery callers; reject `N < 1`. Seam E. `[blocked-by: T1]`
- [ ] T3 Migrate list/show/status/validate/`resolve_change_id` off `changes.join(id)` onto resolve; list+show JSON (and show human) emit `path`. Seam A/B. `[blocked-by: T1]`
- [ ] T4 Graph: remove no-proposal partial-node scan; nodes from discovery only. Seam C. `[blocked-by: T1]`
- [ ] T5 Migrate change lifecycle (start/attach/checkpoint/diff/finalize/archive/git_native/…) to `resolve_change_dir`; archive still flat under `archive/<date>-<id>/`. Seam F. `[blocked-by: T1]`
- [ ] T6 Live specs: add `sdd-workflow` requirement(s) + `.feature` for nested list/show path、conflict、depth、graph、start/archive smoke; `validate --strict --no-check`; update AGENTS/skills wording that `changes/<group>/…/<id>/` is allowed. `[blocked-by: T3,T4,T5]`
- [ ] T7 Integration/BDD bindings as needed + `just check` (or targeted nextest) green. `[blocked-by: T2,T6]`
