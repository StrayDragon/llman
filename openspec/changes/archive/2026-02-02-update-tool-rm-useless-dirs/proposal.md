## Why
`rm-empty-dirs` can delete ignored dependency trees (notably `node_modules`) when `--prune-ignored` is enabled, which is too risky for daily use. The tool should remove clearly useless junk like `__pycache__` while protecting common toolchain data. Users also need explicit configuration to extend or replace the default lists, and we can accept breaking config changes for this tool.

## What Changes
- Rename the primary subcommand to `rm-useless-dirs`, keeping `rm-empty-dirs` as a deprecated alias (behavior unchanged).
- Add toolchain-aware protected directory defaults (Go/Rust/Python/JS/TS/Node + VCS roots) that are never deleted or traversed.
- Keep a minimal useless-directory allowlist (initially `__pycache__`), with explicit config-based extension.
- Introduce `tools.rm-useless-dirs` configuration with `extend` and `override` modes for both protected and useless lists.
- Explicitly drop support for any legacy config keys for this tool (no migration path; breaking change accepted).

## Impact
- Specs: add `tool-rm-useless-dirs` capability with config semantics and defaults.
- Code: `src/tool/command.rs`, `src/tool/rm_empty_dirs.rs`, `src/cli.rs`, `src/tool/config.rs`, `src/config_schema.rs`, `locales/app.yml`.
- Tests: update `tests/rm_empty_dirs_tests.rs` and add coverage for config modes and protected lists.
