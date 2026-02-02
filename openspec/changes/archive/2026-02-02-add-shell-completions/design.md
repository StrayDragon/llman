## Decision
Use runtime generation via `llman self completion --shell <shell>` and print the completion script to stdout. Add `--install` to apply a shell-specific snippet into the appropriate rc/profile file using a marked block, and print the snippet it applied.

## Rationale
- `cargo install` does not provide a standard way to install extra files, so runtime generation works reliably for crates distribution.
- Default output on stdout avoids touching user shell profiles automatically and matches the project's minimal-change principle.
- This approach allows users to re-run the command after upgrades to refresh completions.
- The `--install` output gives a quick copy/paste path and keeps the change idempotent via a marked block.

## Alternatives Considered
- **Build-time generation (`build.rs`)**: would still require a separate install step to place files in shell completion directories; not solved for `cargo install`.
- **Auto-install into shell rc files**: allowed only when explicitly requested via `--install` and gated by confirmation; default remains no writes.

## Installation block strategy
- Use a single marked block so we can detect/update without duplicates:
  - Start marker: `# >>> llman completion >>>`
  - End marker: `# <<< llman completion <<<`
- On `--install`, if a block exists, replace it; if not, append a new block.

## Default rc/profile targets
- bash: first existing of `~/.bashrc`, `~/.bash_profile`, `~/.profile`, otherwise `~/.bashrc`.
- zsh: `~/.zshrc`.
- fish: `~/.config/fish/config.fish`.
- powershell: `$PROFILE` when available, otherwise `~/.config/powershell/Microsoft.PowerShell_profile.ps1`.
- elvish: `~/.elvish/rc.elv`.
- **Packaging-time completion install (brew/apt)**: useful later, but out of scope for crate-only distribution.
