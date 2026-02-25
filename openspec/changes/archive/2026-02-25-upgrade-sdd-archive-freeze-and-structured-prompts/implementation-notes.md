## Implementation Acceptance Record

### Added Commands
- `llman sdd archive run <change-id>`
- `llman sdd archive freeze [--before YYYY-MM-DD] [--keep-recent N] [--dry-run]`
- `llman sdd archive thaw [--change <id> ...] [--dest <path>]`
- Legacy compatibility kept: `llman sdd archive <change-id>`

### Cold Backup Behavior
- Single archive file: `llmanspec/changes/archive/freezed_changes.7z.archived`
- Implementation uses `sevenz-rust2`.
- Freeze append behavior is logical append via merge + atomic replace of the same file path.

### Future Notes Support
- Added template: `llmanspec/templates/spec-driven/future.md` (en + zh-Hans variants).
- Added guidance in new/ff/continue/explore skill/spec-driven templates.
- Missing `future.md` does not block validate/archive.

### Prompt/Skill Improvements
- Added structured protocol section (Context/Goal/Constraints/Workflow/Decision Policy/Output Contract) across skills and spec-driven templates.
- Added new skill: `llman-sdd-specs-compact` (en + zh-Hans).
- `update-skills` template registry updated to generate the new skill.

### Test Evidence
- `cargo +nightly test --test sdd_integration_tests -q` passed.
- `just test` passed.
- `just check-sdd-templates` passed.
- `openspec validate upgrade-sdd-archive-freeze-and-structured-prompts --type change --strict --no-interactive` passed.
