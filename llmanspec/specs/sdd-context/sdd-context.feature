# language: zh-CN
# Partitioned: unique harness only (duplicates removed)
功能: sdd-context

  @req:r79
  场景: rebuild-embeds-feature-scenarios
    假如 某 spec 目录同时含 spec.toon（含 1 个 feature:true 场景）与 1 个 .feature 文件（含 2 个场景）
    当 执行 sdd index rebuild
    那么 tree.json 中该 spec 的 scenarios 含 3 个场景（toon 与 feature 合并去重）
    而且 feature 来源场景的 req_id 为空

  @req:r79
  场景: get-spec-content-returns-spec-level-scenarios
    假如 tree.json 中某 spec 含 req_id 为空的 spec 级别场景
    当 调用 get_spec_content(spec_id, req_ids)
    那么 返回结果含一个 req_id 为空的额外条目
    而且 该条目含全部 spec 级别场景的 given/when/then 全文

  @req:r79
  场景: non-bdd-spec-unchanged
    假如 某 spec 目录无 .feature 文件
    当 执行 sdd index rebuild 后调用检索工具
    那么 scenarios 仅来自 spec.toon
    而且 检索输出与无 feature 嵌入前完全一致

  @req:r58
  场景: rebuild-includes-scenarios
    假如 存在一个 sdd spec，其 spec.toon 含若干 feature:true 的场景
    当 执行 sdd index rebuild --backend pageindex
    那么 生成的 tree.json 中对应 DocNode 的 scenarios 字段非空
    而且 场景含 req_id/id/given/when/then

  @req:r58
  场景: get-document-structure-lists-scenarios
    假如 tree.json 中某 DocNode 含 scenarios 字段
    当 调用 get_document_structure(spec_id)
    那么 返回的每个 req 节点下含 scenarios 列表（仅 id，省 token）

  @req:r58
  场景: get-spec-content-includes-scenarios
    假如 tree.json 中某 DocNode 含 scenarios 字段
    当 调用 get_spec_content(spec_id, req_ids)
    那么 返回的条目含 scenarios 数组
    而且 每个场景含 given/when/then 全文

  @req:r58
  场景: spec-hash-includes-feature
    假如 某 spec 目录下存在 .feature 文件
    当 修改该 .feature 文件后执行 sdd index check
    那么 索引状态为 stale（staleness hash 变化触发）
