## 1. Core behavior
- [x] 1.1 T001: Unify config path resolution (config resolver + CLI/env precedence)
- [x] 1.2 T002: Unify error handling and exit codes (stderr + exit 1 on errors)
- [x] 1.3 T003: Fix cursor export non-interactive selection (db_path/workspace_dir + output_mode validation)

## 2. Tool safety and CI
- [x] 2.1 T004: Safer fallback for clean-useless-comments (skip edits on tree-sitter failure)
- [x] 2.2 T005: Enable clippy gate and fix warnings (CI uses just check)

## 3. Message consistency (can run in parallel once core errors are unified)
- [x] 3.1 T006: Standardize user-facing messages and i18n (English-only placeholders)
- [x] 3.2 T007: Claude Code CLI message consistency
- [x] 3.3 T008: rm-empty-dirs CLI message consistency
- [x] 3.4 T009: prompt CLI message consistency
- [x] 3.5 T010: codex CLI message consistency
- [x] 3.6 T011: clean-comments processor message consistency
- [x] 3.7 T012: cursor database message consistency

## 4. Validation
- [x] 4.1 openspec validate update-cli-quality-specs --strict --no-interactive
