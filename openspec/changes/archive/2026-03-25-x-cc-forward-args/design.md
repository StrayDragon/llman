## Context

目前 `llman x codex` 的主命令支持 `-- <codex-args...>` 透传参数给底层 `codex`；但 `llman x cc` / `llman x claude-code` 的主命令不接受 `--` 后的参数，导致在需要直接调用 `claude` 参数（例如 `--version`、`--help`、`--model ...` 等）时，CLI 体验与 `x codex` 不一致。

本变更只涉及 CLI 参数解析与执行层透传，不改变配置文件格式与安全检查逻辑。

## Goals / Non-Goals

**Goals:**
- 支持 `llman x cc -- <args...>` / `llman x claude-code -- <args...>`，在选择配置组后将参数原样传给 `claude`。
- 与 `llman x codex -- <args...>` 的行为保持一致：必须使用 `--` 进行分隔，避免与子命令解析冲突。
- 保持现有子命令（`account ...`、`run ...`、`stats`、`prompts`）行为不变。

**Non-Goals:**
- 不新增“默认组”或“自动选择唯一配置组”的非交互逻辑。
- 不重构或修改现有安全检查规则与输出格式。
- 不引入新的外部依赖。

## Decisions

- **CLI 解析**：在 `ClaudeCodeArgs` 上增加 `trailing_var_arg` 的 `args: Vec<String>`，并启用 `args_conflicts_with_subcommands = true`，让 `llman x cc -- <args...>` 与子命令互斥，避免歧义。
- **执行透传**：在主命令（未指定子命令）路径中，将解析到的 `args` 追加到 `Command::new("claude")` 的参数列表中；仍然先注入环境变量并执行安全检查。
- **测试策略**：新增 CLI 解析与执行层测试，验证 `--` 后参数被捕获并传递给 `claude`（通过在 `PATH` 注入测试用的 `claude` 可执行脚本来断言参数）。

## Risks / Trade-offs

- **[风险]** 新增 `args_conflicts_with_subcommands` 可能改变某些边界输入的解析错误信息 → **缓解**：明确要求使用 `--` 分隔，并在 help 文案中展示示例。
- **[风险]** 透传任意参数可能触发 `claude` 的不同行为 → **缓解**：行为与用户直接调用 `claude` 保持一致，且仅在用户显式提供 `--` 时启用。
