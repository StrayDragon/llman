## Why
`llman x claude-code` 在交互模式下存在两个可用性/安全提示一致性问题：
- 交互输入参数当前用 `split_whitespace()` 切分，无法正确表达带空格的参数（例如 `--message "hello world"`），导致实际使用受限。
- 安全危险模式匹配对用户配置 patterns 的大小写处理不一致：permission 被 lowercased，但 pattern 未规范化，默认 `contains` 分支可能漏报。

### Current Behavior（基于现有代码）
- 交互 args：`args_text.split_whitespace()`（`src/x/claude_code/command.rs`）。
- pattern 匹配：`permission.to_lowercase()` 后对 `pattern` 走默认 `contains(pattern)`（`src/x/claude_code/security.rs`），pattern 若含大写可能失配。

## What Changes
- 交互 args 输入改为“支持引号/转义”的解析（quote-aware），保证带空格参数可用。
- 危险模式匹配统一为大小写不敏感：内置与用户配置 patterns 采用同一规范化/匹配策略，避免漏报。

### Non-Goals（边界）
- 不改变非交互模式（`-- <args>`）的行为与透传规则。
- 不新增/移除默认危险 patterns，仅修复匹配一致性。

## Impact
- New spec capability: `specs/claude-code-runner/spec.md`
- Affected code: `src/x/claude_code/command.rs`, `src/x/claude_code/security.rs`
