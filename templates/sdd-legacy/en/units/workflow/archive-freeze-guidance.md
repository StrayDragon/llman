<!-- llman-template-version: 1 -->
## Archive Cold Backup Guidance
- If archived directories are growing too large, use cold backup maintenance:
  - Preview freeze candidates: `llman sdd-legacy archive freeze --dry-run`
  - Freeze old archives: `llman sdd-legacy archive freeze --before <YYYY-MM-DD> --keep-recent <N>`
  - Restore when needed: `llman sdd-legacy archive thaw --change <YYYY-MM-DD-id>`
- Apply freeze/thaw only to dated archive directories (`YYYY-MM-DD-*`) and keep a small recent window unfrozen when possible.
