# language: zh-CN
# 对应 spec: config-schemas — 系统 MUST 生成配置 JSON schema 并写入指定路径；MUST 支持
# 以 yaml-language-server 头注释形式写入 schema URL（位于文件顶部）；项目配置 schema
# MUST 为全局子集。
功能: 配置 schema 生成与 YAML LSP 头注释
  @req:r2
  场景: 生成 schema 文件
    假如 用户运行 llman self schema generate
    当 生成完成
    而且 那么全局/项目/llmanspec/sdd-eval playbook schema 被写入或刷新到指定路径

  @req:r2
  场景: 头注释缺失时写入
    假如 用户运行 llman self schema apply 且配置文件缺少 schema 头注释
    当 命令执行
    而且 那么写入对应的 yaml-language-server 头注释

  @req:r2
  场景: 头注释不匹配时修复
    假如 用户运行 llman self schema apply 且 schema URL 与目标不一致
    当 命令执行
    而且 那么修复为正确的 schema URL

  @req:r2
  场景: 项目配置不含全局专用字段
    假如 llman-project-config.schema.json 被生成
    当 检查 schema
    而且 那么不包含 skills.dir
