# Configuration directory

## Default location

On **macOS** and **Linux**, llman uses:

- `~/.config/llman`

You can override this with:

- CLI: `llman --config-dir <path> ...`
- Env: `LLMAN_CONFIG_DIR=<path> llman ...`

Precedence is:

1) `--config-dir`
2) `LLMAN_CONFIG_DIR`
3) default `~/.config/llman`

## macOS legacy compatibility

If you **do not** provide CLI/env overrides on macOS, llman also checks legacy locations:

- `~/Library/Application Support/llman`
- `~/Library/Application Support/com.StrayDragon.llman`

If `~/.config/llman` does **not** contain a recognizable config root, but a legacy location does, llman will:

- resolve to the legacy directory (so existing v1 installs keep working)
- print a migration warning to **stderr** recommending `~/.config/llman`

A “recognizable config root” means the directory contains either:

- `config.yaml`, or
- `prompt/`

## Migration suggestion (manual)

llman does not auto-move files yet. To migrate, copy relevant files from the legacy directory into `~/.config/llman`, for example:

- `config.yaml`
- `prompt/`
- `codex.toml`
- `claude-code.toml`

Then re-run llman and ensure it no longer warns.
