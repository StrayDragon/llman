<!-- llman-template-version: 1 -->
Common commands:
- `llman sdd-legacy list` (list changes)
- `llman sdd-legacy list --specs` (list specs)
- `llman sdd-legacy show <id>` (show change/spec)
- `llman sdd-legacy validate <id>` (validate a change or spec)
- `llman sdd-legacy validate --all` (bulk validate)
- `llman sdd-legacy archive run <id>` (archive a change)
- `llman sdd-legacy archive <id>` (legacy alias of `archive run`)
- `llman sdd-legacy archive freeze [--before YYYY-MM-DD] [--keep-recent N] [--dry-run]` (freeze archived dirs into one cold-backup file)
- `llman sdd-legacy archive thaw [--change <id> ...] [--dest <path>]` (restore from cold-backup file)
