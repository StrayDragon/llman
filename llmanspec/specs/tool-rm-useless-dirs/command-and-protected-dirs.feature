# language: zh-CN
# 对应 spec: tool-rm-useless-dirs — CLI MUST 暴露 rm-useless-dirs 主命令，rm-empty-dirs 为
# 废弃别名（触发同行为并告警）；protected 目录 basenames MUST 在整棵扫描树生效（不删、不遍历）。
功能: 命令别名与整棵树生效的 protected 目录
  @req:r1
  场景: 新命令执行清理
    假如 用户运行 llman tool rm-useless-dirs
    当 命令执行
    而且 那么以既有选项集执行清理行为

  @req:r1
  场景: 废弃别名执行并告警
    假如 用户运行 llman tool rm-empty-dirs
    当 命令执行
    而且 那么执行清理行为
    而且 而且打印引用 rm-useless-dirs 的废弃告警

  @req:r1
  场景: 即使 enabled prune-ignored 也保留 protected 目录
    假如 node_modules/ 经 .gitignore 忽略且运行启用 --prune-ignored -y
    当 清理执行
    而且 那么node_modules 及其内容保持完好

  @req:r1
  场景: 空的 protected 目录仍被保留
    假如 存在一个空的 protected 目录
    当 清理执行
    而且 那么该目录仍被保留
    而且 而且不被移除

  @req:r1
  场景: 扫描遇到 protected 组件不进入子树
    假如 扫描遇到 {protected_subpath}
    当 清理执行
    而且 那么不进入该 protected 子树
    而且 而且不删除其下任何内容
