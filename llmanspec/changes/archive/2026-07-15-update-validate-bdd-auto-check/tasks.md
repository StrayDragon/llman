# Tasks

- [x] 1. Fix `has_spec_files()`: recognize `.feature` files in delta spec dirs (BDD-on change detection)
- [x] 2. CLI: add `--no-check` flag to `Validate` args in `src/sdd/command.rs`
- [x] 3. CLI: auto-enable check mode when `bdd.run_command` is configured; `--check` becomes no-op alias
- [x] 4. Bulk validation: wire check mode when bdd config exists in `run_bulk_validation()`
- [x] 5. Spec validation: when `bdd.run_command` configured and `--no-check` not passed, run BDD runner
- [x] 6. Deprecation: emit INFO when `--check` passed but BDD-off
- [x] 7. Tests: add scenario bindings in `tests/bdd_steps.rs` for new feature scenarios (SKIP: recursive — validate calls BDD runner; binding would self-trigger)
- [x] 8. `just check-all` — format, lint, test pass
