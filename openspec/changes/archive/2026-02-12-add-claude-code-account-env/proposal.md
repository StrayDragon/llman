## Why

Claude Code 的 group 配置（`claude-code.toml`）里通常包含一组需要注入的环境变量。现在这些 env 只能通过 `run/use` 运行子进程时注入，难以在 shell 脚本/CI 中复用。新增一个 `account env` 命令可以把选中 group 的键值对转成可直接执行的 env 注入语句，避免手工拷贝与出错。

## What Changes

- 新增命令 `llman x claude-code account env <GROUP>`：读取 `<GROUP>` 的配置并将其所有键值对输出为当前系统可执行的 env 注入语句（Linux/macOS 输出 `export KEY='value'`；Windows 输出 PowerShell `$env:KEY='value'`）。
- 新增别名路径支持：`llman x cc account env <GROUP>`。
- 输出顺序稳定（按 key 排序），值会被安全引用/转义，便于 `eval/source/Invoke-Expression` 等方式消费。
- 当 `<GROUP>` 不存在或配置为空时，给出明确错误信息并以非零退出。

## Capabilities

### New Capabilities

（无）

### Modified Capabilities

- `claude-code-account-management`: 增加 `account env <GROUP>` 子命令，用于从 `claude-code.toml` 的 groups 中生成 shell/PowerShell 可执行的 env 注入语句。

## Impact

- CLI：`src/x/claude_code/command.rs` 增加新的 `AccountAction` 分支与输出逻辑
- 可能新增：用于生成 shell/PowerShell 引用的通用 helper（便于测试与复用）
- 测试：新增集成测试覆盖输出格式、排序、转义、group 不存在等行为（使用 `LLMAN_CONFIG_DIR` 与临时目录）
