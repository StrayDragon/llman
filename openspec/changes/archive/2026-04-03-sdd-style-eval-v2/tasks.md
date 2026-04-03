## 1. v2 Fixture (format-sensitive)

- [x] 1.1 Create new fixture dir `agentdev/promptfoo/sdd_llmanspec_styles_v2/` with promptfooconfig + prompts + tests
- [x] 1.2 Add v2 task prompt that forces `Read` of main + delta `spec.md` and requires a file-level edit
- [x] 1.3 Add deterministic baseline seeding step in runner to pre-create a non-trivial capability + change in each workspace

## 2. v2 Hard Gate Assertions

- [x] 2.1 Add Python assertion(s) for v2 that verify at least one `Read` of `llmanspec/specs/**/spec.md` occurred
- [x] 2.2 Add Python assertion(s) that verify the edited spec still contains the correct style fence (` ```ison/toon/yaml `)
- [x] 2.3 Keep `llman sdd validate --all --strict --no-interactive` as the final hard gate per workspace

## 3. Runner Enhancements (batch + aggregation)

- [x] 3.1 Add `--fixture v1|v2` (default v1) to `agentdev/promptfoo/run-sdd-claude-style-eval.sh`
- [x] 3.2 Introduce a batch root directory for `--runs N` so all runs share one parent and can be aggregated
- [x] 3.3 Generate `meta/aggregate.{json,md}` at batch level: per-style pass rate + mean/median/p90 for tokens/turns/cost

## 4. CLI Entrypoints + Docs

- [x] 4.1 Update `scripts/sdd-claude-style-eval.sh` and `justfile` to expose `--fixture` and a `--aggregate` friendly mode
- [x] 4.2 Document how to interpret v1 vs v2 results (what each is measuring) in `agentdev/promptfoo/**/README.md`

## 5. Verification

- [x] 5.1 Run `just sdd-claude-style-eval --fixture v2 --cc-account <acct> --runs 1` and ensure 3/3 pass
- [x] 5.2 Run `just sdd-claude-style-eval --fixture v2 --cc-account <acct> --runs 3` and ensure aggregate report is created
