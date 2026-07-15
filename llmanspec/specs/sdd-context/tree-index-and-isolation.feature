# language: zh-CN
# 对应 spec: sdd-context — pageindex 的 sdd index rebuild 从已解析 IR 直接构建树索引，
# 无需 LLM 抽取；检索经三工具 agentic loop；不同 backend 索引隔离存储且不互相降级。
功能: 树索引构建、agentic 工具导航与后端隔离
  场景: rebuild 从 IR 构建树索引无需 LLM
    假如 存在多个已解析的 sdd spec
    当 执行 sdd index rebuild --backend pageindex
    那么 生成 tree.json
    而且构建过程不发起任何 LLM 请求

  场景: agentic loop 经三工具导航
    假如 chat 模型收到 system prompt 与三工具（list_specs / get_document_structure / get_spec_content）
    当 sdd context 检索执行
    那么 LLM 依次调用工具后输出 direct/related 分类结果

  场景: 后端隔离不互相降级
    假如 .context/rag/ 已有索引但 .context/pageindex/ 不存在
    当 执行 sdd context --backend pageindex
    那么 返回 quality 为 unavailable 且 errorKind 为 index_missing
    而且不降级使用 rag 索引
