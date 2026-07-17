# language: zh-CN
# 对应 spec: upgrade-guide — 每个 feature 条目 MUST 含 suggestion 字段（agent 可直接展示的
# 精确 YAML 片段）；命令 MUST 只读不写；全部 feature 已启用且模板最新时 MUST 省略 suggestions 表
# 并在 footer 指明无可用升级。
功能: suggestion 含 YAML 片段、只读执行与全启用输出
  @req:r1
  场景: suggestion 含可展示的 YAML 片段
    假如 某 feature 在项目配置中处于禁用状态
    当 查看该 feature 条目
    而且 那么suggestion 字段含合法 YAML 片段
    而且 而且agent 可直接向用户展示

  @req:r1
  场景: 命令只读执行不修改任何文件
    假如 命令对任意项目运行
    当 命令执行
    而且 那么不创建、修改或删除任何文件

  @req:r1
  场景: 全部 feature 已启用时省略 suggestions 表
    假如 全部 feature 已启用且模板为最新
    当 命令执行
    而且 那么省略 suggestions 表
    而且 而且footer 指明无可用升级
