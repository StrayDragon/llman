# language: zh-CN
# managed by llman sdd partition-migrate
功能: tool-rm-useless-dirs

  @req:r1
  场景: 新命令执行清理
    假如 用户运行 llman tool rm-useless-dirs
    当 命令执行
    那么 以既有选项集执行清理行为

  @req:r1
  场景: 废弃别名执行并告警
    假如 用户运行 llman tool rm-empty-dirs
    当 命令执行
    那么 执行清理行为
    而且 打印引用 rm-useless-dirs 的废弃告警

  @req:r1
  场景: 即使 enabled prune-ignored 也保留 protected 目录
    假如 node_modules/ 经 .gitignore 忽略且运行启用 --prune-ignored -y
    当 清理执行
    那么 node_modules 及其内容保持完好

  @req:r1
  场景: 空的 protected 目录仍被保留
    假如 存在一个空的 protected 目录
    当 清理执行
    那么 该目录仍被保留
    而且 不被移除

  @req:r1
  场景: 扫描遇到 protected 组件不进入子树
    假如 扫描遇到 {protected_subpath}
    当 清理执行
    那么 不进入该 protected 子树
    而且 不删除其下任何内容

  @req:r1
  场景: 移除 __pycache__
    假如 target 含 a/__pycache__/b.pyc 且运行 live（-y）
    当 清理执行
    那么 a/__pycache__ 被移除

  @req:r1
  场景: 移除 .pytest_cache
    假如 target 含 .pytest_cache 且运行 live（-y）
    当 清理执行
    那么 移除 .pytest_cache

  @req:r1
  场景: extend 模式合并默认与配置
    假如 配置设置 protected.mode=extend 且 names: [".idea"]
    当 解析 protected 列表
    那么 含默认项与 .idea

  @req:r1
  场景: override 模式替换默认
    假如 配置设置 protected.mode=override 且 names: []
    当 解析 protected 列表
    那么 不应用任何默认 protected 名称

  @req:r1
  场景: legacy 配置键存在时加载失败
    假如 配置含 tools.rm-empty-dirs
    当 加载配置
    那么 加载失败并报告该 legacy 键不被支持

  @req:r1
  场景: 非 CWD target 使用自己的 gitignore
    假如 用户运行 llman tool rm-useless-dirs {target} 且未传 --gitignore
    当 解析默认 gitignore
    那么 若 {target}/.gitignore 存在则使用它
    而且 不隐式使用调用者 CWD 的 .gitignore

  @req:r1
  场景: 扫描子目录时使用仓库根 gitignore
    假如 用户运行 llman tool rm-useless-dirs {sub_target} 且未传 --gitignore
    而且 {repo_root}/.git 存在
    当 解析默认 gitignore
    那么 若 {repo_root}/.gitignore 存在则使用它
