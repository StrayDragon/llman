## Context

llman 已支持 `llman x claude-code` 的 account 管理（`edit/list/import/use`）与 `run --group`。配置文件 `claude-code.toml` 的每个 group 本质上就是一组环境变量键值对（`HashMap<String, String>`），目前仅在运行 `claude` 子进程时通过 `Command::env` 注入。

为了让这份 group 配置能在 shell 脚本/CI 中复用，需要一个只做“把 group 转成可执行 env 注入语句并输出到 stdout”的命令。

## Goals / Non-Goals

**Goals:**
- 提供 `llman x claude-code account env <GROUP>`（以及 `x cc` 别名）输出 env 注入语句
- 输出语法随平台选择：Linux/macOS 输出 POSIX `export ...`；Windows 输出 PowerShell `$env:...`
- 输出稳定可预测：按 key 排序；每行一条赋值
- 输出安全：value 必须被引用/转义；key 必须校验为安全的 env var 名称，避免注入
- 无副作用：不写入任何文件，不修改配置，不启动 claude

**Non-Goals:**
- 不做跨 shell 的格式矩阵（例如 fish/elvish），仅按 OS 输出默认格式
- 不提供自动写入 shell profile/rc 的安装能力
- 不对敏感值做遮罩（这是“生成可执行注入语句”的命令，遮罩会破坏可用性）

## Decisions

1. **输出内容仅包含可执行语句**
   - 成功时 stdout 只输出注入脚本内容：以 `#` 开头的用法注释 + env 注入语句（可直接用于 `eval "$(…)"` / `source <(… )` / `… | Out-String | Invoke-Expression`）
   - 失败时使用非零退出码，并将错误写到 stderr

2. **平台选择**
   - `cfg!(windows)` 选择 PowerShell 输出：`$env:KEY='value'`
   - 非 Windows 输出 POSIX：`export KEY='value'`

3. **稳定排序**
   - 将 group 的 key 排序后再输出，避免 HashMap 迭代顺序导致 diff/脚本不稳定

4. **安全引用与校验**
   - **key 校验**：仅允许匹配 `^[A-Za-z_][A-Za-z0-9_]*$` 的 key；否则命令失败并报告非法 key 列表（防止 `export FOO=$(rm -rf /)` 这类注入）
   - **POSIX value quoting**：使用单引号包裹，内部单引号用 `'\''` 形式转义
   - **PowerShell value quoting**：使用单引号包裹，内部单引号按 PowerShell 规则替换为 `''`

## Risks / Trade-offs

- [敏感信息泄露] 输出的注入语句包含密钥 → 文档/帮助中提示避免直接打印到日志，并建议通过管道立即消费
- [用户期望 `source $(...)`] bash/zsh 需要 `source <(… )` 或 `eval "$(…)"` 才能消费 stdout → 在命令 help/示例中给出正确用法
- [Windows 支持差异] PowerShell 环境变量名规则与 POSIX 不完全一致 → 通过 key 校验统一约束，减少歧义
