## Git-native lifecycle (brief)

Do not conflate **skill navigation** with the **Git-native lifecycle**. Full diagram: root `AGENTS.md` or the diagram inside `llman-sdd-propose`.

Hard rules:
1. **First** Branch binding (`change start` / `attach`) → Full; **then** Specs landing (edit and commit `llmanspec/specs/**` on the bound branch).
2. No live contract edits → `needs_specs_change: false`. Apply requires `readyToImplement=true`.
3. Close-out: `change finalize` (auto commit `archive(sdd): <id>`; `--no-commit` to skip). `change checkpoint` is removed (calling it exits non-zero and points to finalize).
4. **Do not** commit live specs on the default branch; if already attached, do not re-run `start`.
