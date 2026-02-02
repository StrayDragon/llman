## Why
- CLI subcommands are growing and manual typing is slow/error-prone.
- clap already supports generating shell completion scripts; exposing this improves day-to-day usability with minimal risk.

## What Changes
- Add a new `llman self completion` subcommand that prints a completion script to stdout.
- Support shell targets: bash, zsh, fish, powershell, elvish.
- Add `--install` to update the appropriate shell rc/profile file with a marked completion block (idempotent: update in place if present, do not duplicate).
- `--install` prints the exact snippet it applied so the user can copy/paste if desired.
- Writes to shell rc files are explicit and gated by user confirmation.

## Impact
- Specs: `cli-experience`.
- Code: `src/cli.rs`, `src/self_command.rs`, `Cargo.toml`, `locales/app.yml` (new help/messages), optional docs.
