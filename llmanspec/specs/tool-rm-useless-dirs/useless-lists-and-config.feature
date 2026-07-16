# language: zh-CN
# 对应 spec: tool-rm-useless-dirs — useless allowlist 目录即使非空也 MUST 移除；protected/useless
# 列表可经 tools.rm-useless-dirs 配置（mode: extend/override）；legacy 配置键 MUST 被拒绝；
# 默认 gitignore MUST 基于扫描目标所属仓库/目标自身解析（而非调用者 CWD）。
功能: useless 列表移除、列表可配置、legacy 键拒绝与 gitignore 解析
  @req:r1
  场景: 移除 __pycache__
    假如 target 含 a/__pycache__/b.pyc 且运行 live（-y）
    当 清理执行
    而且 那么a/__pycache__ 被移除

  @req:r1
  场景: 移除 .pytest_cache
    假如 target 含 .pytest_cache 且运行 live（-y）
    当 清理执行
    而且 那么移除 .pytest_cache

  @req:r1
  场景: extend 模式合并默认与配置
    假如 配置设置 protected.mode=extend 且 names: [".idea"]
    当 解析 protected 列表
    而且 那么含默认项与 .idea

  @req:r1
  场景: override 模式替换默认
    假如 配置设置 protected.mode=override 且 names: []
    当 解析 protected 列表
    而且 那么不应用任何默认 protected 名称

  @req:r1
  场景: legacy 配置键存在时加载失败
    假如 配置含 tools.rm-empty-dirs
    当 加载配置
    而且 那么加载失败并报告该 legacy 键不被支持

  @req:r1
  场景: 非 CWD target 使用自己的 gitignore
    假如 用户运行 llman tool rm-useless-dirs {target} 且未传 --gitignore
    当 解析默认 gitignore
    而且 那么若 {target}/.gitignore 存在则使用它
    而且 而且不隐式使用调用者 CWD 的 .gitignore

  @req:r1
  场景: 扫描子目录时使用仓库根 gitignore
    假如 用户运行 llman tool rm-useless-dirs {sub_target} 且未传 --gitignore
    而且 {repo_root}/.git 存在
    当 解析默认 gitignore
    而且 那么若 {repo_root}/.gitignore 存在则使用它
