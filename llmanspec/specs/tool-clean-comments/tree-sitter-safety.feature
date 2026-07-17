# language: zh-CN
# 对应 spec: tool-clean-comments — tree-sitter 不可用或对某文件失败时，clean-comments
# processor MUST 跳过该文件、记录错误并继续处理其它文件。
功能: tree-sitter 不可用时安全跳过
  @req:r3
  场景: tree-sitter 无法初始化时不修改任何文件
    假如 tree-sitter 无法初始化
    当 clean-comments processor 运行
    那么 不修改任何文件
    而且 报告错误

  @req:r3
  场景: 某文件 tree-sitter 失败时该文件保持不变并继续其它文件
    假如 tree-sitter 在处理特定文件时失败
    当 clean-comments processor 继续运行
    那么 该文件保持不变
    而且 继续处理剩余文件
