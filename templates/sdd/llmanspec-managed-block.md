# LLMAN Spec-Driven Development

These instructions apply to AI assistants working in this project.

When a request:
- mentions proposal/spec/change/plan
- introduces a new feature, breaking change, architecture shift, or large performance/security work
- is ambiguous and needs authoritative specs

Use the llmanspec workflow and the `llman sdd` commands.

Quick commands:
- `llman sdd list`
- `llman sdd show <item>`
- `llman sdd validate <id> --strict --no-interactive`
- `llman sdd archive <id>`

Keep this managed block so `llman sdd update` can refresh it.
