## Context
`rm-empty-dirs` removes empty directories and, with `--prune-ignored`, deletes ignored entries to allow pruning ignored-only trees. This has caused accidental removal of dependency directories such as `node_modules`. The goal is to make cleanup safer, align the name with intent (`rm-useless-dirs`), and allow users to customize behavior while accepting breaking config changes for this tool.

## Goals / Non-Goals
- Goals:
  - Provide a safer default by protecting common toolchain directories across Go/Rust/Python/JS/TS/Node, VCS metadata, and IDE metadata.
  - Remove known junk directories (starting with Python caches) even when non-empty.
  - Allow users to either extend or fully override defaults for both protected and useless lists.
  - Keep the implementation small and predictable.
- Non-Goals:
  - Build a complex rules engine or auto-detect project language stacks.
  - Delete arbitrary build outputs by default (e.g., `dist/`, `build/`).

## Decisions
- Decision: Introduce `rm-useless-dirs` as the primary command and keep `rm-empty-dirs` as a deprecated alias that warns.
  - Reason: Improves clarity without immediate CLI breakage.
- Decision: Add a toolchain-aware protected list and treat these directories as untouchable (no traversal, no deletion) even with `--prune-ignored`.
  - Reason: Prevents accidental removal of dependency, environment, and IDE directories.
- Decision: Expand the default useless list to include common Python cache directories.
  - Reason: Matches the intent to remove useless directories beyond emptiness.
- Decision: Add `tools.rm-useless-dirs` configuration with per-list `mode: extend|override`.
  - Reason: Gives users a clear, explicit way to tune behavior.
- Decision: Drop support for any legacy config keys for this tool (breaking change accepted). If a legacy key is present, parsing/validation must fail with a clear error.
  - Reason: Avoids silent misconfiguration and aligns with the request to fully remove old config.

## Default Lists (Basename match)
Protected (never delete or traverse):
- VCS: `.git`, `.hg`, `.svn`, `.bzr`
- IDE: `.idea`, `.vscode`
- Node/JS/TS: `node_modules`, `.yarn`, `.pnpm-store`, `.pnpm`, `.npm`
- Python: `.venv`, `venv`, `.tox`, `.nox`, `__pypackages__`
- Rust: `target`, `.cargo`
- Go: `vendor`

Useless (remove even if non-empty):
- `__pycache__`
- `.pytest_cache`
- `.mypy_cache`
- `.ruff_cache`
- `.basedpyright`
- `.pytype`
- `.pyre`
- `.ty`
- `.ty_cache`
- `.ty-cache`

## Configuration Shape (proposed)
```yaml
version: "0.1"
tools:
  rm-useless-dirs:
    protected:
      mode: extend   # extend|override
      names: ["node_modules", ".venv"]
    useless:
      mode: extend   # extend|override
      names: [".pytest_cache"]
```
- `extend`: union of defaults + user names.
- `override`: use only the user-provided list.

## Risks / Trade-offs
- The protected list may feel conservative; override mode provides escape hatches.
- Expanding useless defaults increases automation; override mode allows opting out.
- Breaking config behavior (legacy keys) requires explicit user updates.

## Migration Plan
- Add new config section and defaults.
- Keep CLI alias but reject legacy config keys if present.
- Update tests and i18n strings.
