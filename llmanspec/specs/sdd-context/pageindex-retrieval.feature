# language: zh-CN
# 对应 spec: sdd-context r27 — pageindex 后端与配置隔离；无 chat model 时 api_error。
# 索引 stale/missing 的懒刷新见 r97（context-lazy-rebuild.feature）；不再因 missing 直接 unavailable。
功能: pageindex agentic 检索与配置提示
  @req:r27
  场景: 配置缺失时给出简洁可操作提示
    假如 LLMAN_SDD_INDEX_CHAT_MODEL 未设置
    当 执行 sdd context（索引已 fresh 或已可懒刷新）
    那么 返回 api_error 或 quality=unavailable
    而且 提示明确列出需设置的变量名
    而且 而且不回退到任何其它检索后端
    而且 而且内容简洁无冗余
