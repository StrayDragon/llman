# Proposal: Harden git base-ref and env injection

## Why

QA 审查发现两类可利用面：

1. `LLMANSPEC_BASE_REF` 原样进入 git argv，无 `--` 隔离、不以 `-` 开头校验，存在 **git option injection**（CI / 共享 runner 风险）。
2. Claude Code / Codex 将配置组中**全部** env 键注入子进程；`env_injection` 只校验键名语法，不拦 `LD_PRELOAD` / `PATH` / `DYLD_*` 等，恶意或共享 `*.toml` 可劫持加载器。

这两项都会改变外部可观测的拒绝行为，且 Codex 现有合约写明「注入所有键值对」，因此必须走完整 SDD（不可 quick）。

## What Changes

- **sdd-workflow**：收紧 `LLMANSPEC_BASE_REF` 安全校验；git 调用对该参数做 argv 隔离。
- **codex-account-management**：将 r3 从「注入所有键」改为「注入安全键 + denylist 拒绝危险键并失败」。
- **claude-code-runner**：新增子进程 env denylist 合约（与 Codex 对齐）。

## Capabilities

| Capability | Delta |
|------------|--------|
| `sdd-workflow` | `modify_requirement` r15 |
| `codex-account-management` | `modify_requirement` r3 |
| `claude-code-runner` | `add_requirement` r4 |

## Impact

- **Breaking（有意）**：配置里若故意设置 `PATH`/`LD_PRELOAD` 等危险键，原先会注入，现在会失败。
- **Breaking（有意）**：`LLMANSPEC_BASE_REF=-c` 等 option-like 值将报错。
- 正常 API key / 自定义 `FOO_BAR` 类键不受影响。
- 无配置路径迁移；无 Windows 专项扩展。

## Ethics

- `ethics.risk_level`: medium（安全硬化，改变既有注入/校验行为）
- `ethics.prohibited_actions`: 不得在未更新合约的情况下静默跳过危险键并继续执行
- `ethics.required_evidence`: 拒绝路径的单元/集成测试；validate 通过
