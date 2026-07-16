# language: zh-CN
# 对应 spec: codex-account-management — 编辑器命令 MUST 支持 $VISUAL/$EDITOR 含参数；
# 切换组时 MUST 将 provider 配置 upsert 到 ~/.codex/config.toml 并设置顶层 model_provider，
# 支持 override_name 覆盖 effective_name。
功能: 编辑器参数支持与 provider 配置 upsert
  @req:r1
  场景: 编辑器含参数时正确执行
    假如 $EDITOR 设置为 code --wait 且用户运行 llman x codex account edit
    当 命令执行
    而且 那么执行 code --wait <codex.toml-path>
    而且 而且若编辑器非零退出则返回错误

  @req:r1
  场景: 首次切换写入 provider 无 override_name
    假如 用户选择 minimax 组且 codex config 无 model_providers.minimax
    当 切换组
    而且 那么写入 minimax provider 配置
    而且 而且设置 model_provider = "minimax"

  @req:r1
  场景: 存在 override_name 时按 effective_name 写入
    假如 用户选 b 组且配置含 override_name = "a"
    而且 而且codex config 无 model_providers.a
    当 切换组
    而且 那么写入到 model_providers.a
    而且 而且设置 model_provider = "a"
    而且 而且写入项 name = "a"

  @req:r1
  场景: 透传额外字段且不写入 llman_configs
    假如 选定 provider table 含额外字段且存在 override_name
    当 切换组
    而且 那么codex config 中保留该额外字段
    而且 而且不包含 llman_configs 子表

  @req:r1
  场景: 基于 effective_name 幂等不重复写入
    假如 用户再次选 b 组（override_name = "a"）
    而且 而且codex config 已 model_provider = "a" 且 model_providers.a 配置一致
    当 切换组
    而且 那么检测到配置已存在且一致
    而且 而且跳过写入
