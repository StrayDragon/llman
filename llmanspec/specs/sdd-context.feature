# language: zh-CN
# capability: sdd-context
# purpose: 规范 `llman sdd context` 命令的后端选项、默认值与 pageindex 检索行为。
# scope: llmanspec/specs/sdd-context.feature

功能: sdd-context

  @req:r27
  规则: pageindex backend and config isolation
    llman sdd index/context MUST only support `--backend pageindex`. Chat model and embedding model configuration MUST be separated. The default chat host MUST be a safe empty value: when unset (and without OpenAI host fallback), ChatConfig MUST error with guidance to set `LLMAN_SDD_INDEX_CHAT_API_HOST` rather than routing to an implicit endpoint.
  @req:r58
  规则: Scenario-Aware Retrieval Partitioned
    build_docs 与检索工具 MUST 以 *.feature 为唯一规格内容源暴露场景：compute_spec_hash MUST 仅哈希各 <capability>.feature（遗留 spec.toon MUST 被静默忽略，不参与哈希、不报错）。get_document_structure MUST 能列出规则块（@req 句柄）下嵌套场景的 id。get_spec_content MUST 返回对应 given/when/then 全文且同一 scenario id MUST NOT 出现两份正文。旧 tree.json 无 scenarios 字段 MUST 仍可加载（缓存结构兼容）。
  @req:r79
  规则: Feature Embedding 单次且 feature 优先
    index_rebuild MUST 解析并编入全部 *.feature 的嵌套场景。req_id 取自所属规则块的 @req 句柄；规则块缺 @req 时该 spec 在 validate 中按结构门报 ERROR（spec-format r132）。畸形 .feature MUST 跳过并警告而非中止 rebuild；遗留 spec.toon MUST 被静默忽略（不编入、不告警、不中止其余 capability 的编入）。
  @req:r97
  规则: context 对 stale/missing 懒刷新
    llman sdd context 在 pageindex 索引为 stale 或 missing 时 MUST 自动执行一次 index rebuild（无需 chat model）后再进行 retrieval，MUST NOT 仅因 stale 或 missing 返回 status.quality=unavailable 或 errorKind index_stale/index_missing。索引 corrupted 时 MUST 尝试 rebuild；rebuild 失败则非零或 JSON error。chat model 未配置时，在成功 rebuild 后仍可按既有 api_error 语义失败。
