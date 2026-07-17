# language: zh-CN
# 对应 spec: codex-account-management — env 注入 MUST 拒绝危险键（LD_PRELOAD/LD_LIBRARY_PATH/
# DYLD_*/PATH 及大小写变体），拒绝时不启动 codex；import 交互式创建 provider；
# 主命令/run 支持 -- 透传；account 提供 edit 与 import。
功能: 环境变量安全、交互导入与命令透传
  @req:r44
  场景: 注入 LD_PRELOAD 危险键被拒绝
    假如 所选组 env 含 LD_PRELOAD={evil_path}
    当 用户切换到该组并触发 codex 子进程启动
    而且 那么命令失败并报告危险环境变量被拒绝
    而且 而且未启动 codex

  @req:r44
  场景: 交互导入创建 provider
    假如 用户运行 llman x codex account import 并输入 minimax / https://api.minimax.com/v1 / MINIMAX_KEY / sk-xxx
    当 命令执行
    而且 那么在 codex.toml 中创建 model_providers.minimax 及其 env 子表

  @req:r44
  场景: 透传 codex 参数
    假如 用户运行 llman x codex -- --help -m o3 并在交互中选任意 provider
    当 系统执行
    而且 那么执行 codex --help -m o3

  @req:r44
  场景: 未提供透传参数时仅执行 codex
    假如 用户运行 llman x codex 并在交互中选任意 provider
    当 系统执行
    而且 那么执行 codex

  @req:r44
  场景: 非交互 run
    假如 用户运行 llman x codex run --group openai -- --help
    当 系统执行
    而且 那么upsert openai provider 到 codex config
    而且 而且注入环境变量
    而且 而且执行 codex --help

  @req:r44
  场景: 交互 run
    假如 用户运行 llman x codex run -i
    当 系统执行
    而且 那么交互选组、询问参数
    而且 而且然后 upsert + 注入 + 执行

  @req:r44
  场景: account 默认进入编辑
    假如 用户运行 llman x codex account
    当 系统执行
    而且 那么使用编辑器打开 codex.toml
