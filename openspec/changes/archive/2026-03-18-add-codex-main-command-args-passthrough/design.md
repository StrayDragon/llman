## Context

`llman x codex` 的主命令路径（无子命令）用于交互选择 provider，并在同步配置 + 注入环境变量后执行 `codex`。当前该路径无法接收并透传 `--` 之后的参数，导致用户无法用一条命令完成“选组 + 运行 codex 子命令/flag”，只能绕行到 `llman x codex run -i` 再二次输入参数。

## Goals / Non-Goals

**Goals:**
- 支持 `llman x codex -- <codex-args...>`，在交互选组后将 `<codex-args...>` 原样注入到实际 `codex` 执行参数中。
- 参数透传按 argv token 级别进行，不执行 shell，不做字符串级解析，避免注入风险。
- 不影响 `llman x codex run` / `account` / `stats` 的现有行为。

**Non-Goals:**
- 不新增或改变 codex 的参数语义；llman 不解释 `--` 之后的任何参数。
- 不支持“无 `--` 时也自动把未知参数当作 codex 参数”的宽松解析（避免与子命令/未来参数冲突）。

## Decisions

1. **使用 clap 的 trailing var arg 捕获 `--` 后参数**
   - 在 `CodexArgs`（主命令层级）新增一个 `Vec<String>` positional 参数，启用 `trailing_var_arg`。
   - 通过 `args_conflicts_with_subcommands` 约束该 positional 只在“无子命令”时生效，避免与 `run/account/stats` 解析歧义。

2. **复用现有执行路径**
   - 透传参数直接进入现有 `activate_and_exec(...)` 的 `args`，保持同步 provider + 注入 env 的逻辑不变。

3. **测试策略以 CLI 解析为主**
   - 新增单测覆盖 `llman x codex -- --help -m o3` 能正确解析并保留 codex args。
   - 保留并不修改 `llman x codex run --group ... -- <args>` 的既有解析测试。

## Risks / Trade-offs

- **解析歧义风险** → 通过 `args_conflicts_with_subcommands` 避免“既有子命令又有主命令透传 args”的混用。
- **用户误用把 `llman` 的参数放到 `--` 后** → 在 help/错误信息中保持清晰描述（仅透传给 codex）。

