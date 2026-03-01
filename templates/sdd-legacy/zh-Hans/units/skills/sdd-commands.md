<!-- llman-template-version: 1 -->
常用命令：
- `llman sdd-legacy list`（列出变更）
- `llman sdd-legacy list --specs`（列出 specs）
- `llman sdd-legacy show <id>`（查看 change/spec）
- `llman sdd-legacy validate <id>`（校验变更或 spec）
- `llman sdd-legacy validate --all`（批量校验）
- `llman sdd-legacy archive run <id>`（归档变更）
- `llman sdd-legacy archive <id>`（`archive run` 的兼容别名）
- `llman sdd-legacy archive freeze [--before YYYY-MM-DD] [--keep-recent N] [--dry-run]`（将已归档目录冻结到单一冷备文件）
- `llman sdd-legacy archive thaw [--change <id> ...] [--dest <path>]`（从冷备文件恢复目录）
