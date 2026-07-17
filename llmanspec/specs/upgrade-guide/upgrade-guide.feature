# language: zh-CN
# managed by llman sdd partition-migrate
功能: upgrade-guide

  @req:r1
  场景: suggestion 含可展示的 YAML 片段
    假如 某 feature 在项目配置中处于禁用状态
    当 查看该 feature 条目
    那么 suggestion 字段含合法 YAML 片段
    而且 agent 可直接向用户展示

  @req:r1
  场景: 命令只读执行不修改任何文件
    假如 命令对任意项目运行
    当 命令执行
    那么 不创建、修改或删除任何文件

  @req:r1
  场景: 全部 feature 已启用时省略 suggestions 表
    假如 全部 feature 已启用且模板为最新
    当 命令执行
    那么 省略 suggestions 表
    而且 footer 指明无可用升级

  @req:r1
  场景: 输出 TOON 文档
    假如 命令对配置不完整的项目运行
    当 命令执行
    那么 输出为合法 TOON 文档
    而且 kind 为 llman.sdd.upgrade_guide
    而且 含 features 表

  @req:r1
  场景: 新增 feature 须登记到 FEATURES 常量
    假如 开发者新增一个 SDD 配置字段或 skill
    当 准备合并
    那么 开发者 MUST 在 upgrade_guide.rs 的 FEATURES 常量中加入对应 FeatureDef 条目
