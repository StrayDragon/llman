# language: zh-CN
# 对应 spec: cli r8-r10 — context / index 命令与索引新鲜度协议。
功能: context 与 index 命令行为
  @req:r8
  场景: context 在有索引时返回 semantic 质量
    假如 agent 调用 llman sdd context --task XDG_CONFIG_HOME --paths src/config.rs
    当 命令执行完成
    那么 返回 quality=semantic 且 direct 含 config-paths

  @req:r8
  场景: context 在无索引时返回 unavailable
    假如 agent 在缺少 embedding 索引时调用 llman sdd context
    当 命令执行完成
    那么 返回 quality=unavailable 并含重建索引提示

  @req:r9
  场景: index rebuild --check 不调用 API
    假如 用户运行 llman sdd index rebuild --check
    当 命令执行完成
    那么 输出 Index status 且不发起 embedding API 调用

  @req:r10
  场景: context 在索引过期时降级为 keyword
    假如 agent 在索引 staleness 过期时调用 llman sdd context
    当 命令执行完成
    那么 使用 keyword 检索且 quality=keyword
