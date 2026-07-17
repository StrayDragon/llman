# language: zh-CN
# 对应 spec: tool-clean-comments — regex 回退 MUST NOT 默认启用（仅留作未来显式 opt-in）；
# 且当用户显式传入路径时，非文件输入（如目录）MUST 被显式跳过并提示，而非当成文件读取失败。
功能: regex 回退默认禁用且非文件输入被显式处理
  @req:r69
  场景: 默认运行不使用 regex 回退
    假如 clean-comments 在无显式 opt-in 的情况下运行
    当 processor 选择移除路径
    那么 不使用 regex 回退移除

  @req:r69
  场景: 目录输入被跳过并提示
    假如 用户运行 llman tool clean-useless-comments {path}
    当 输入为目录
    那么 工具提示该目录输入被跳过
    而且 而且不会把它当成文件读取失败
    而且 而且继续处理其它输入
