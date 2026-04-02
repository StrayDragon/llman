## 1. v3 Fixture (spec-only realistic workflow)

- [ ] 1.1 Create new fixture dir `agentdev/promptfoo/sdd_llmanspec_styles_v3/` with promptfooconfig + prompts + tests
- [ ] 1.2 Add v3 task prompt: TODO app spec-only scenario with 3 change cycles (propose -> apply -> archive -> commit) in one workspace
- [ ] 1.3 Add deterministic baseline seeding for v3 (TODO baseline + 3 change skeletons with markers) in the runner

## 2. v3 Hard Gate Assertions

- [ ] 2.1 Add Python assertion(s) that enforce required `Read` of style-specific `llmanspec/**/spec.md` before edits
- [ ] 2.2 Add Python assertion(s) that enforce marker replacement for all 3 change cycles (file-level edits required even if CLI add-* is used)
- [ ] 2.3 Add Python assertion(s) that verify `llman sdd archive run` is executed for all 3 changes and changes are actually archived
- [ ] 2.4 Keep `llman sdd validate --all --strict --no-interactive` as the final hard gate per workspace

## 3. Runner + Aggregation Enhancements (v3 + judge score-only)

- [ ] 3.1 Extend runner to support `--fixture v3` (default remains v1)
- [ ] 3.2 Make `--judge claude` score-only (must not affect pass/fail); keep hard gate as source of truth
- [ ] 3.3 Extend per-run summary and batch aggregate reports to include judge score distributions per style (mean/median/p90)

## 4. Docs / Entrypoints

- [ ] 4.1 Document v3 intent and how it differs from v1/v2 in `agentdev/promptfoo/**/README.md`
- [ ] 4.2 Add/refresh examples for running v3 with judge + multi-run aggregate

## 5. Verification

- [ ] 5.1 Run `just sdd-claude-style-eval --fixture v3 --cc-account <acct> --runs 1` and ensure 3/3 pass
- [ ] 5.2 Run `just sdd-claude-style-eval --fixture v3 --cc-account <acct> --runs 3 --judge claude` and ensure aggregate report includes judge score stats
