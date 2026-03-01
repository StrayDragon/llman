# LLMAN Spec-Driven Development

These instructions apply to AI assistants working in this project.

When a request:
- mentions proposal/spec/change/plan
- introduces a new feature, breaking change, architecture shift, or large performance/security work
- is ambiguous and needs authoritative specs

Use the llmanspec workflow and the `llman sdd-legacy` commands.

Quick commands:
- `llman sdd-legacy list`
- `llman sdd-legacy show <item>`
- `llman sdd-legacy validate <id> --strict --no-interactive`
- `llman sdd-legacy archive <id>`

Keep this managed block so `llman sdd-legacy update` can refresh it.
