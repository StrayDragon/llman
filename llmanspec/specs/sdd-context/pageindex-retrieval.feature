# language: zh-CN
# 对应 spec: sdd-context — sdd context MUST 使用 pageindex agentic 推理检索；
# 索引缺失/过期或模型未配置时返回 quality=unavailable 且不回退；配置缺失时给出可操作提示。
功能: pageindex agentic 检索与不可用时不回退
  场景: 索引缺失时返回 unavailable 且不回退
    假如 pageindex 索引缺失
    当 执行 sdd context（未显式 --backend）
    那么 返回 quality=unavailable
    而且 qualityNote 含 rebuild 命令
    而且不回退到任何其它检索方式

  场景: 配置缺失时给出简洁可操作提示
    假如 LLMAN_SDD_INDEX_CHAT_MODEL 未设置
    当 执行 sdd context
    那么 提示明确列出需设置的变量名与 rebuild 命令
    而且内容简洁无冗余
