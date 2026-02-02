## 1. Implementation
- [x] 1.1 Update tool CLI wiring to expose `rm-useless-dirs` and keep `rm-empty-dirs` as a deprecated alias with a warning.
- [x] 1.2 Implement protected list handling and useless allowlist removal, with protected directories never traversed/deleted.
- [x] 1.3 Add `tools.rm-useless-dirs` configuration support with `extend|override` modes and legacy-key rejection.
- [x] 1.4 Update schema generation for global/project configs to include the new tool config.
- [x] 1.5 Update user-facing messages and i18n keys for the new command name, deprecation warning, and config errors.
- [x] 1.6 Add tests for protected directories, useless removal, and config override/extend behavior.

## 2. Validation
- [x] 2.1 Run `just test` (or `cargo +nightly test --all`) with `LLMAN_CONFIG_DIR=./artifacts/testing_config_home` if needed.
- [x] 2.2 Run `openspec validate update-tool-rm-useless-dirs --strict --no-interactive`.
