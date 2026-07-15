# language: zh-CN
# 对应 spec: sdd-specs-compaction-guidance — 当前版本 MUST NOT 实现 specs compact CLI
# 子命令；压缩治理 MUST 以代码与 specs 为事实源（而非已废弃的 ISON 制品）；
# 且 MUST 包含压缩前后安全回归比对步骤。
功能: specs 压缩 CLI 预留未实现且治理基于事实源并含安全门
  场景: 当前版本不暴露 specs compact 子命令
    假如 用户执行 llman sdd --help
    当 查看帮助
    那么 帮助中不出现 specs compact 子命令

  场景: 压缩决策以代码与 specs 为事实源
    假如 用户按新风格遵循 specs 压缩指引
    当 做 keep/merge/remove 决策
    那么 决策以代码与 specs 事实源为准

  场景: 压缩流程包含压缩前后安全回归门
    假如 已准备好一份压缩方案
    当 执行压缩工作流
    那么 流程包含压缩前后输出的安全回归比对步骤
