## Why

目前 `llman x codex` 的主命令路径（无子命令）是“交互选组 → 同步 provider → 注入 env → 执行 `codex`”。但它无法接收并透传 `--` 之后的 codex 参数：

- 用户想要在一次命令里完成“选组 + 运行 codex 子命令/flag”（例如 `llman x codex -- --yolo`、`llman x codex -- --help`）。
- 现状只能绕行到 `llman x codex run -i` 并在交互里再输入参数，既冗长也不利于脚本化/复用。

## What Changes

- 支持 `llman x codex -- <codex-args...>`：
  - `--` 之后的所有参数视为 codex 参数，llman 不解释、不解析，只做原样透传（按 argv token 级别）。
  - 用户交互选完组后，llman 执行 `codex <codex-args...>`（保持现有：provider upsert + env 注入）。
- 未提供 `-- <args>` 时，`llman x codex` 行为保持不变（选组后直接执行 `codex`）。

### Non-Goals（边界）

- 不引入 shell 执行或字符串级解析；只透传 clap 接收到的 argv tokens（与现有 `Command::arg` 一致），避免注入风险。
- 不改变 `codex` 自身的参数语义；llman 只负责“选组 + 注入 + exec”。

## Capabilities

### New Capabilities
- （none）

### Modified Capabilities
- `codex-account-management`: 扩展 `llman x codex` 主命令支持 `-- <codex-args...>` 透传，并在交互选组后将其注入到实际的 `codex` 执行参数中。

## Impact

- Code: `src/x/codex/command.rs`
  - clap 参数模型：为主命令路径增加 trailing args 捕获（`-- <args>`）。
  - 执行路径：将捕获到的 args 传入 `activate_and_exec`，确保真正影响 `Command::new("codex")` 的 `.arg(...)`。
- Tests:
  - 增加 CLI parse 用例：`llman x codex -- --help -m o3` 能被解析并保留为 codex args。
