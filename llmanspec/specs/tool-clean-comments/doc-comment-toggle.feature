# language: zh-CN
# 对应 spec: tool-clean-comments — doc comment 移除开关（docstrings/jsdoc/doc-comments/godoc）
# MUST 与其它注释开关语义一致：true 启用移除、false 禁用移除、None 默认禁用（保留）。
功能: doc comment 开关语义一致
  场景: 开关为 true 时启用 doc comment 移除
    假如 配置对某语言设置 docstrings: true 且文件包含 doc comments
    当 规则判断是否移除
    那么 doc comments 可被移除（满足其它条件时）

  场景: 未配置开关时默认保留 doc comments
    假如 配置未指定 doc comment 开关
    当 规则判断是否移除
    那么 doc comments 被保留
