## MODIFIED Requirements

### Requirement: 编辑器命令必须支持参数
当打开配置文件进行编辑时，Codex account manager MUST 支持 `$VISUAL` 或 `$EDITOR` 包含参数（例如 `code --wait`）。实现 MUST 执行解析后的命令，并将 `codex.toml` 配置文件路径作为最后一个参数追加。

#### Scenario: editor 包含参数
- **WHEN** `$EDITOR` 设置为 `code --wait` 且用户运行 `llman x codex account edit`
- **THEN** 命令执行 `code --wait <codex.toml-path>`；若编辑器以非零退出则返回错误

## ADDED Requirements

### Requirement: Provider 配置 upsert 到 codex config
切换组时，系统 MUST 将选中组的 `[model_providers.<name>]` 字段（不含 `.env` 子表）upsert 到 `~/.codex/config.toml`，同时设置顶层 `model_provider = "<name>"`。系统 MUST NOT 删除 codex config 中的其他已有配置。

#### Scenario: 首次切换写入 provider
- **WHEN** 用户选择 minimax 组且 `~/.codex/config.toml` 中无 `model_providers.minimax`
- **THEN** 系统将 minimax 的 provider 配置写入 codex config 并设置 `model_provider = "minimax"`

#### Scenario: 重复切换不重复写入
- **WHEN** 用户再次选择 minimax 组且 codex config 中已有相同配置
- **THEN** 系统检测到配置已存在且一致，跳过写入

### Requirement: 环境变量注入执行
切换组时，系统 MUST 将 `[model_providers.<name>.env]` 中的所有键值对作为环境变量注入 codex 子进程。

#### Scenario: 注入 API Key
- **WHEN** 用户选择 minimax 组，其 env 中有 `MINIMAX_CODEX_API_KEY = "sk-xxx"`
- **THEN** codex 子进程的环境中包含 `MINIMAX_CODEX_API_KEY=sk-xxx`

### Requirement: Import 交互式创建 provider
`llman x codex account import` MUST 交互式询问 group_name、base_url、env_key_id（默认 CODEX_API_KEY）、api_key_value，并将结果写入 `codex.toml`。

#### Scenario: 交互导入
- **WHEN** 用户运行 `llman x codex account import` 并输入 minimax / https://api.minimax.com/v1 / MINIMAX_KEY / sk-xxx
- **THEN** 系统在 codex.toml 中创建 `[model_providers.minimax]` 和 `[model_providers.minimax.env]`

### Requirement: Run 命令支持交互和非交互模式
`llman x codex run` MUST 支持 `--group <name>` 非交互模式和 `-i` 交互模式。

#### Scenario: 非交互 run
- **WHEN** 用户运行 `llman x codex run --group openai -- --help`
- **THEN** 系统 upsert openai provider 到 codex config，注入环境变量，执行 `codex --help`

#### Scenario: 交互 run
- **WHEN** 用户运行 `llman x codex run -i`
- **THEN** 系统交互选组、询问参数，然后 upsert + 注入 + 执行

### Requirement: Account 命令提供 edit 和 import
`llman x codex account` MUST 默认进入 edit。`edit` MUST 用编辑器打开 codex.toml，`import` MUST 交互式创建新 provider。

#### Scenario: account 默认编辑
- **WHEN** 用户运行 `llman x codex account`
- **THEN** 系统使用编辑器打开 codex.toml

