## Why
`llman x codex account edit` 当前将 `$EDITOR` 当作单一可执行文件名处理，导致常见配置（例如 `code --wait`、`vim -u ...`）无法使用，降低可用性。

### Current Behavior（基于现有代码）
- `Command::new(&editor)` 直接使用环境变量字符串作为程序名（`src/x/codex/command.rs`），未做拆分与引号处理。

## What Changes
- 支持 `$VISUAL`/`$EDITOR` 包含参数的形式（quote-aware 拆分为 command + args）。
- 继续保留无环境变量时的 fallback（默认 `vi`）。

### Non-Goals（边界）
- 不自动执行 shell（避免注入风险），只做安全的参数拆分与调用。

## Impact
- New spec capability: `specs/codex-account-management/spec.md`
- Affected code: `src/x/codex/command.rs`
