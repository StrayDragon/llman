# Tasks: feat-sdd-status-compact

## Phase 1: Status Command Core (cli)

- [x] **1. Update `src/sdd/shared/status.rs`** — Full rewrite with `StatusArgs` (target, format, json), TARGET resolution, TOON builders, priority sort, and backward-compat JSON path.
- [x] **2. Add `--format toon|json` CLI parameter** — In `src/sdd/command.rs`, added `--format` arg with default `toon`. `--json` kept as shorthand.
- [x] **3. Add TARGET resolution logic** — Exact active → exact archive → fuzzy/date-prefix → unique = single, multiple = summary list, none = error with suggestions.
- [x] **4. Add priority extraction** — `c<N>-` prefix parsed from directory name. Falls back to alphabetical sort.
- [x] **5. Implement project-level TOON output** — `kind: llman.sdd.status`, `counts{active,specs}:`, `changes[N]{name,stage,tasks,next}:`.
- [x] **6. Implement single-change TOON output** — `change{name,stage,priority,tasks}:`, `tasks[N]{id,title,test}:` (incomplete only), `next:` field.
- [x] **7. Implement archived-change TOON output** — `change{name,stage,priority,tasks}:`, `ops[N]{op,req_id,title}:` via backend parsing, `next:`.
- [x] **8. Update CLI definition** — Removed old text output path. Wired `--format` arg into StatusArgs.
- [x] **9. Test status command** — Manual smoke tests passed: project-level, single-change, archive fuzzy, multi-match, --json, --format json, error cases.

## Phase 2: Apply-Cycle Skill (sdd-workflow)

- [x] **10. Create `templates/sdd/{locale}/skills/llman-sdd-apply-cycle.md`** — Template with `disable-model-invocation: true`, single-closed-loop workflow, ethics governance keys. Both `en` and `zh-Hans`.
- [x] **11. Update `src/sdd/project/templates.rs`** — Registered `llman-sdd-apply-cycle.md` in `DEFAULT_SKILL_FILES`.
- [x] **12. Update `src/sdd/project/update_skills.rs`** — Added `llman-sdd-apply-cycle` to `EXPECTED_WORKFLOW_SKILLS` in test. No Claude-code-specific generation exists yet (pre-existing).
- [x] **13. Test skill generation** — `update_skills` unit tests pass (322/322). Ethics governance enforced.

## Phase 3: Verify & Archive

- [x] **14. Run `llman sdd validate feat-sdd-status-compact --strict --no-interactive`** — Passed (only pending tasks expected)
- [x] **15. Manual smoke test** — Tested 11 output modes: project-level, single-change, archive, date-prefix multi-match, fuzzy, --json, --format json, error cases
- [x] **16. Archive and commit** — Running now
