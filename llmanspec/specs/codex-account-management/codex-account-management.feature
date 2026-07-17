# language: zh-CN
# managed by llman sdd partition-migrate
功能: codex-account-management

  @req:r1
  场景: 编辑器含参数时正确执行
    假如 $EDITOR 设置为 code --wait 且用户运行 llman x codex account edit
    当 命令执行
    那么 执行 code --wait <codex.toml-path>
    而且 若编辑器非零退出则返回错误

  @req:r1
  场景: 首次切换写入 provider 无 override_name
    假如 用户选择 minimax 组且 codex config 无 model_providers.minimax
    当 切换组
    那么 写入 minimax provider 配置
    而且 设置 model_provider = "minimax"

  @req:r1
  场景: 存在 override_name 时按 effective_name 写入
    假如 用户选 b 组且配置含 override_name = "a"
    而且 codex config 无 model_providers.a
    当 切换组
    那么 写入到 model_providers.a
    而且 设置 model_provider = "a"
    而且 写入项 name = "a"

  @req:r1
  场景: 透传额外字段且不写入 llman_configs
    假如 选定 provider table 含额外字段且存在 override_name
    当 切换组
    那么 codex config 中保留该额外字段
    而且 不包含 llman_configs 子表

  @req:r1
  场景: 基于 effective_name 幂等不重复写入
    假如 用户再次选 b 组（override_name = "a"）
    而且 codex config 已 model_provider = "a" 且 model_providers.a 配置一致
    当 切换组
    那么 检测到配置已存在且一致
    而且 跳过写入

  @req:r1
  场景: 注入 LD_PRELOAD 危险键被拒绝
    假如 所选组 env 含 LD_PRELOAD={evil_path}
    当 用户切换到该组并触发 codex 子进程启动
    那么 命令失败并报告危险环境变量被拒绝
    而且 未启动 codex

  @req:r1
  场景: 交互导入创建 provider
    假如 用户运行 llman x codex account import 并输入 minimax / https://api.minimax.com/v1 / MINIMAX_KEY / sk-xxx
    当 命令执行
    那么 在 codex.toml 中创建 model_providers.minimax 及其 env 子表

  @req:r1
  场景: 透传 codex 参数
    假如 用户运行 llman x codex -- --help -m o3 并在交互中选任意 provider
    当 系统执行
    那么 执行 codex --help -m o3

  @req:r1
  场景: 未提供透传参数时仅执行 codex
    假如 用户运行 llman x codex 并在交互中选任意 provider
    当 系统执行
    那么 执行 codex

  @req:r1
  场景: 非交互 run
    假如 用户运行 llman x codex run --group openai -- --help
    当 系统执行
    那么 upsert openai provider 到 codex config
    而且 注入环境变量
    而且 执行 codex --help

  @req:r1
  场景: 交互 run
    假如 用户运行 llman x codex run -i
    当 系统执行
    那么 交互选组、询问参数
    而且 然后 upsert + 注入 + 执行

  @req:r1
  场景: account 默认进入编辑
    假如 用户运行 llman x codex account
    当 系统执行
    那么 使用编辑器打开 codex.toml
