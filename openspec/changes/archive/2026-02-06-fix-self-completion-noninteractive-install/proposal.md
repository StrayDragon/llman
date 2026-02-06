## Why
`llman self completion --install` 当前在非交互环境下直接失败（要求 `stdin` 是 TTY），导致无法在 CI/初始化脚本中完成自动化安装。我们希望在保持默认“必须确认”的前提下，提供一个**显式且安全**的绕过方式。

### Current Behavior（基于现有代码）
- `confirm_install` 在非交互时直接返回错误（`src/self_command.rs`），没有 `--yes` 类似的显式同意开关。

## What Changes
- 为 completion install 新增显式 `--yes`（或等价）：
  - 非交互：未传 `--yes` 必须拒绝写入并返回错误；传 `--yes` 则直接写入。
  - 交互：默认仍提示确认；传 `--yes` 则跳过提示。

### Non-Goals（边界）
- 不改变 completion script 的生成内容与 marker block 的幂等更新策略。

## Impact
- Affected specs: `specs/cli-experience/spec.md`
- Affected code: `src/self_command.rs`
