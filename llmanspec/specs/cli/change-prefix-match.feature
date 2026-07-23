# language: zh-CN
# 对应 spec: cli r112 — 所有接收 change 名的命令支持前缀匹配解析
# 优先级：精确活跃 > 前缀活跃 > 前缀归档
功能: Change 名前缀匹配
  @req:r112
  场景: 精确匹配活跃 change 优先命中
    假如 活跃 change 中有一个名为 c123-fix-bug 的变更
    而且 还有一个名为 c456-add-feature 的变更
    当 用完整 id "c123-fix-bug" 运行 llman sdd show
    那么 输出包含 c123-fix-bug 的内容

  @req:r112
  场景: 前缀唯一匹配活跃 change
    假如 活跃 change 中只有一个以 "c123" 开头的变更
    当 用前缀 "c123" 运行 llman sdd show
    那么 输出包含对应变更的内容

  @req:r112
  场景: 前缀匹配归档 change（活跃无匹配时）
    假如 活跃 change 中无 "fix" 开头的变更
    而且 归档 change 中有一个以 "fix" 开头的变更
    当 用前缀 "fix" 运行 llman sdd show
    那么 输出包含归档变更的内容

  @req:r112
  场景: 多前缀匹配报错
    假如 活跃 change 中有 c123-foo 和 c123-bar 两个以 "c123" 开头的变更
    当 用前缀 "c123" 运行 llman sdd show
    那么 命令报错并列出两个候选项

  @req:r112
  场景: 无匹配时报错
    假如 无任何 change 以 "zzz" 开头
    当 用前缀 "zzz" 运行 llman sdd show
    那么 命令报错并提示 "zzz" 未找到

  @req:r112
  场景: status 也使用相同前缀匹配规则
    假如 活跃 change 中有 c789-update 变更
    当 用前缀 "c789" 运行 llman sdd status
    那么 输出 c789-update 的变更 TOON

  @req:r112
  场景: validate 也使用相同前缀匹配规则
    假如 活跃 change 中有 c789-update 变更
    当 用前缀 "c789" 运行 llman sdd validate
    那么 校验运行在 c789-update 上

  @req:r112
  场景: graph 也使用相同前缀匹配规则
    假如 活跃 change 中有 c789-update 变更
    当 用前缀 "c789" 运行 llman sdd graph
    那么 依赖图以 c789-update 为中心

  @req:r112
  场景: change archive 也使用相同前缀匹配规则
    假如 活跃 change 中有 c789-update 变更
    当 用前缀 "c789" 运行 llman sdd change archive
    那么 归档操作运行在 c789-update 上
