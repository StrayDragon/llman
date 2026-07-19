# language: zh-CN
# 对应 spec: sdd-context r97 — context 在 index stale/missing 时自动 rebuild 再检索，
# 不再仅因 stale/missing 返回 unavailable。（与 r27「无 chat model → api_error」并存。）
功能: context 索引懒刷新
  @req:r97
  场景: index stale 时 context 自动 rebuild
    假如 pageindex 索引存在但相对 specs 已 stale
    当 执行 sdd context
    那么 MUST NOT 仅因 stale 返回 errorKind index_stale
    而且 索引被重建后再进入 retrieval（无 chat model 时可随后 api_error）

  @req:r97
  场景: index missing 时 context 自动 rebuild
    假如 pageindex 索引缺失
    当 执行 sdd context
    那么 MUST NOT 仅因 missing 返回 errorKind index_missing
    而且 先 rebuild 再 retrieval
