## 1. Preparation
- [x] 1.1 Confirm rc/profile target paths per shell and `--install` snippet formats.

## 2. Implementation
- [x] 2.1 Add `clap_complete` dependency and wire supported shell enum.
- [x] 2.2 Implement `llman self completion --shell <shell>` to generate to stdout.
- [x] 2.3 Implement `--install` to confirm and update rc/profile files with a marked block.
- [x] 2.4 Ensure install is idempotent (update existing block, do not duplicate).
- [x] 2.5 Add localized strings for new command/help text and example usage snippets.
- [x] 2.6 Update any CLI docs/help references that list available self commands.

## 3. Verification
- [x] 3.1 Manual: `LLMAN_CONFIG_DIR=./artifacts/testing_config_home llman self completion --shell bash` outputs a script.
- [x] 3.2 Manual: repeat for zsh/fish/powershell/elvish.
- [x] 3.3 Manual: `--install` prompts before writing and prints the snippet.
- [x] 3.4 Manual: re-run `--install` and confirm no duplicate block is added.
- [x] 3.5 `just test` (or `cargo +nightly test --all`) if the change touches shared CLI wiring.
