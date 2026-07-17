# language: zh-CN
# managed by llman sdd partition-migrate
功能: tool-clean-comments

  @req:r1
  场景: 开关为 true 时启用 doc comment 移除
    假如 配置对某语言设置 docstrings: true 且文件包含 doc comments
    当 规则判断是否移除
    那么 doc comments 可被移除（满足其它条件时）

  @req:r1
  场景: 未配置开关时默认保留 doc comments
    假如 配置未指定 doc comment 开关
    当 规则判断是否移除
    那么 doc comments 被保留

  @req:r1
  场景: 默认运行不使用 regex 回退
    假如 clean-comments 在无显式 opt-in 的情况下运行
    当 processor 选择移除路径
    那么 不使用 regex 回退移除

  @req:r1
  场景: 目录输入被跳过并提示
    假如 用户运行 llman tool clean-useless-comments {path}
    当 输入为目录
    那么 工具提示该目录输入被跳过
    而且 不会把它当成文件读取失败
    而且 继续处理其它输入

  @req:r1
  场景: tree-sitter 无法初始化时不修改任何文件
    假如 tree-sitter 无法初始化
    当 clean-comments processor 运行
    那么 不修改任何文件
    而且 报告错误

  @req:r1
  场景: 某文件 tree-sitter 失败时该文件保持不变并继续其它文件
    假如 tree-sitter 在处理特定文件时失败
    当 clean-comments processor 继续运行
    那么 该文件保持不变
    而且 继续处理剩余文件
