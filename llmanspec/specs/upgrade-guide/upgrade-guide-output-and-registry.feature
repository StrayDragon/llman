# language: zh-CN
# 对应 spec: upgrade-guide — upgrade-guide 命令 MUST 输出 kind 为 llman.sdd.upgrade_guide 的
# TOON 文档（含 features 表与 suggestions 表）；所有可升级 SDD feature MUST 登记在 FEATURES 常量中。
功能: upgrade-guide 输出格式与 feature 注册表单一真源
  @req:r2
  场景: 输出 TOON 文档
    假如 命令对配置不完整的项目运行
    当 命令执行
    而且 那么输出为合法 TOON 文档
    而且 而且kind 为 llman.sdd.upgrade_guide
    而且 而且含 features 表

  @req:r2
  场景: 新增 feature 须登记到 FEATURES 常量
    假如 开发者新增一个 SDD 配置字段或 skill
    当 准备合并
    而且 那么开发者 MUST 在 upgrade_guide.rs 的 FEATURES 常量中加入对应 FeatureDef 条目
