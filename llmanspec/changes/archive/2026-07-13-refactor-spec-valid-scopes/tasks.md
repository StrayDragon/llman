# Tasks: refactor-spec-valid-scopes

## 1. Artifacts

- [x] proposal / design / delta（sdd-workflow r47）
- [x] `llman sdd validate refactor-spec-valid-scopes --strict --no-interactive`

## 2. Apply valid_scope map

对下列 specs 将 `valid_scope` 从 `src/,tests/` 改为指定列表（含 `llmanspec/specs/<id>/`）：

- [x] agent-tools-usage-stats → `src/usage_stats/,src/x/codex/stats.rs,src/x/claude_code/stats.rs,src/x/cursor/stats.rs,src/x/cursor/database.rs,tests/codex_stats_tests.rs,tests/claude_code_stats_tests.rs,tests/cursor_stats_tests.rs,llmanspec/specs/agent-tools-usage-stats/`
- [x] claude-code-account-management → `src/x/claude_code/command.rs,src/x/claude_code/config.rs,src/x/claude_code/env_injection.rs,src/editor.rs,src/env_safety.rs,tests/claude_code_account_edit_tests.rs,tests/claude_code_account_env_tests.rs,llmanspec/specs/claude-code-account-management/`
- [x] claude-code-runner → `src/x/claude_code/command.rs,src/x/claude_code/interactive.rs,src/x/claude_code/security.rs,src/x/claude_code/env_injection.rs,src/arg_utils.rs,src/env_safety.rs,tests/claude_code_forward_args_tests.rs,llmanspec/specs/claude-code-runner/`
- [x] cli → `src/sdd/command.rs,src/cli.rs,llmanspec/specs/cli/`
- [x] cli-experience → `src/self_command.rs,src/lib.rs,src/main.rs,locales/app.yml,llmanspec/specs/cli-experience/`
- [x] codex-account-management → `src/x/codex/command.rs,src/x/codex/config.rs,src/x/codex/interactive.rs,src/editor.rs,src/env_safety.rs,llmanspec/specs/codex-account-management/`
- [x] codex-agents-management → `src/x/codex/agents.rs,src/x/codex/command.rs,src/x/codex/prompts.rs,src/managed_block.rs,llmanspec/specs/codex-agents-management/`
- [x] config-paths → `src/config.rs,src/cli.rs,tests/config_tests.rs,tests/print_config_dir_path_tests.rs,tests/path_validation_tests.rs,llmanspec/specs/config-paths/`
- [x] config-schemas → `src/config_schema.rs,src/self_command.rs,artifacts/schema/,justfile,llmanspec/specs/config-schemas/`
- [x] cursor-claude-ignore-sync → `src/tool/sync_ignore.rs,src/tool/command.rs,src/x/cursor/command.rs,src/x/claude_code/command.rs,llmanspec/specs/cursor-claude-ignore-sync/`
- [x] cursor-export → `src/x/cursor/command.rs,src/x/cursor/database.rs,src/x/cursor/models.rs,llmanspec/specs/cursor-export/`
- [x] dependency-upgrade-workflow → `Cargo.toml,Cargo.lock,rust-toolchain.toml,justfile,.github/workflows/ci.yaml,llmanspec/specs/dependency-upgrade-workflow/`
- [x] errors-exit → `src/main.rs,src/error.rs,src/cli.rs,tests/error_tests.rs,locales/app.yml,llmanspec/specs/errors-exit/`
- [x] nightly-toolchain-governance → `rust-toolchain.toml,.github/workflows/ci.yaml,justfile,llmanspec/specs/nightly-toolchain-governance/`
- [x] prompts-management → `src/prompts/,src/cli.rs,src/x/cursor/prompts.rs,src/x/codex/prompts.rs,src/x/claude_code/prompts.rs,src/managed_block.rs,tests/prompts_orchestrator_tests.rs,llmanspec/specs/prompts-management/`
- [x] sdd-ab-evaluation → `agentdev/promptfoo/,agentdev/docker/,scripts/sdd-prompts-eval.sh,scripts/sdd-claude-style-eval.sh,justfile,llmanspec/specs/sdd-ab-evaluation/`
- [x] sdd-archive-freeze → `src/sdd/change/freeze.rs,src/sdd/command.rs,templates/sdd/en/units/workflow/archive-freeze-guidance.md,templates/sdd/zh-Hans/units/workflow/archive-freeze-guidance.md,llmanspec/specs/sdd-archive-freeze/`
- [x] sdd-context → `src/sdd/context/,src/sdd/command.rs,llmanspec/specs/sdd-context/`
- [x] sdd-eval-acp-pipeline → `artifacts/schema/playbooks/,llmanspec/specs/sdd-eval-acp-pipeline/`
- [x] sdd-eval-workflow-dsl → `artifacts/schema/playbooks/en/llman-sdd-eval.schema.json,llmanspec/specs/sdd-eval-workflow-dsl/`
- [x] sdd-future-changes → `templates/sdd/en/units/skills/future-planning.md,templates/sdd/zh-Hans/units/skills/future-planning.md,src/sdd/shared/validate.rs,src/sdd/change/archive.rs,llmanspec/specs/sdd-future-changes/`
- [x] sdd-ison-authoring → `src/sdd/authoring/,src/sdd/spec/,src/sdd/command.rs,templates/sdd/,llmanspec/specs/sdd-ison-authoring/`
- [x] sdd-ison-pipeline → `src/sdd/spec/,src/sdd/project/templates.rs,src/sdd/shared/,templates/sdd/,llmanspec/specs/sdd-ison-pipeline/`
- [x] sdd-legacy-compat → `src/sdd/project/templates.rs,src/sdd/command.rs,templates/sdd/,llmanspec/specs/sdd-legacy-compat/`
- [x] sdd-multi-style-formats → `src/sdd/spec/,src/sdd/project/config.rs,src/sdd/project/migrate.rs,src/sdd/project/init.rs,src/sdd/command.rs,llmanspec/specs/sdd-multi-style-formats/`
- [x] sdd-openspec-interop → `src/sdd/project/interop.rs,src/sdd/command.rs,llmanspec/specs/sdd-openspec-interop/`
- [x] sdd-specs-compaction-guidance → `templates/sdd/en/skills/llman-sdd-specs-compact.md,templates/sdd/zh-Hans/skills/llman-sdd-specs-compact.md,src/sdd/project/update_skills.rs,src/sdd/project/templates.rs,llmanspec/specs/sdd-specs-compaction-guidance/`
- [x] sdd-structured-skill-prompts → `templates/sdd/,src/sdd/project/templates.rs,src/sdd/project/update_skills.rs,llmanspec/specs/sdd-structured-skill-prompts/`
- [x] sdd-template-units-and-jinja → `src/sdd/project/templates.rs,templates/sdd/,scripts/check-sdd-templates.py,justfile,llmanspec/specs/sdd-template-units-and-jinja/`
- [x] sdd-workflow → `src/sdd/,templates/sdd/,scripts/check-sdd-templates.py,llmanspec/specs/sdd-workflow/`
- [x] skills-management → `src/skills/,tests/skills_integration_tests.rs,tests/skills_targets_sync_tests.rs,llmanspec/specs/skills-management/`
- [x] tests-ci → `.github/workflows/ci.yaml,justfile,tests/,llmanspec/specs/tests-ci/`
- [x] tool-clean-comments → `src/tool/clean_comments.rs,src/tool/processor.rs,src/tool/tree_sitter_processor.rs,src/tool/command.rs,src/tool/config.rs,tests/processor_tests.rs,tests/tree_sitter_tests.rs,tests/tool_tests.rs,llmanspec/specs/tool-clean-comments/`
- [x] tool-rm-useless-dirs → `src/tool/rm_empty_dirs.rs,src/tool/command.rs,src/tool/config.rs,tests/rm_empty_dirs_tests.rs,llmanspec/specs/tool-rm-useless-dirs/`

## 3. Gates

- [x] `llman sdd validate --all --strict --no-interactive`（目标：无 STALE 误报于本分支已触及路径）
- [x] `just qa`（或至少 clippy + 相关测试）
