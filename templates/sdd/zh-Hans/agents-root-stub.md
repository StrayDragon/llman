# LLMAN 规范驱动开发

本仓库使用 llman SDD，项目上下文和规则在 `llmanspec/config.yaml`。

常用命令:
- `llman sdd list`
- `llman sdd show <item>`
- `llman sdd validate <id> --strict --no-interactive`
- `llman sdd archive <id>`
- `llman sdd update-skills --all`

Spec 格式: TOON (```toon 代码块)。

保留此托管块，便于 `llman sdd update` 刷新。
