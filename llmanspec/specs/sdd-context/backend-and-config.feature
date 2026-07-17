# language: zh-CN
# 对应 spec: sdd-context — llman sdd index/context MUST 仅支持 --backend pageindex；
# chat 模型与 embedding 模型分离配置；默认 chat host 为安全空值。
功能: pageindex 为唯一检索后端且配置分离
  @req:r27
  场景: 拒绝非 pageindex 后端值
    假如 用户执行 sdd context --backend {bad_backend}
    当 命令解析 --backend 值
    那么 报错退出
    而且 而且提示 backend 仅支持 pageindex
    而且 而且引导配置所需环境变量

  @req:r27
  场景: chat 模型 endpoint 回退复用 OPENAI 值
    假如 LLMAN_SDD_INDEX_CHAT_API_HOST 未设置但 LLMAN_SDD_INDEX_OPENAI_API_HOST 已设置
    当 解析 chat 模型 endpoint
    那么 回退复用 LLMAN_SDD_INDEX_OPENAI_API_HOST 的值

  @req:r27
  场景: 默认 host 为空且未配置时报错
    假如 LLMAN_SDD_INDEX_CHAT_API_HOST 未设置且 LLMAN_SDD_INDEX_OPENAI_API_HOST 也未设置
    当 ChatConfig::from_env() 执行
    那么 返回错误并提示用户配置 LLMAN_SDD_INDEX_CHAT_API_HOST
    而且 而且不回退到任何可路由地址

  @req:r27
  场景: 显式 host 正常生效
    假如 LLMAN_SDD_INDEX_CHAT_API_HOST 已设置为合法 URL
    当 ChatConfig::from_env() 执行
    那么 正常返回配置
    而且 而且使用用户提供的值
