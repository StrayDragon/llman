# language: zh-CN
# 对应 spec: sdd-workflow r10/r47/r48 — list/context/triage 冒烟场景。
功能: list、context 与 triage 冒烟
  @req:r39
  场景: list --specs --json 含元数据字段
    假如 agent 运行 llman sdd list --specs --json
    当 命令执行完成
    那么 输出含 purpose、validScope、health、staleness

  @req:r47
  场景: context direct 规格须全文阅读
    假如 context 返回 config-paths 位于 direct
    当 agent 收到 context 输出
    那么 agent 阅读 config-paths 规格全文

  @req:r48
  场景: 行为合约变更走完整 SDD
    假如 agent 收到会改变退出码行为的任务
    当 进行变更规模判断
    那么 选择完整 SDD 流程（proposal + specs + tasks）

  @req:r48
  场景: 实现级改动走快速路径
    假如 agent 收到仅修复 README 错字的任务
    当 进行变更规模判断
    那么 选择快速路径且不创建 change 目录

  @req:r60
  场景: show spec 分段展示 Constraints 与 Harness
    假如 llman 二进制已构建
    当 运行 llman sdd show errors-exit --type spec
    那么 stdout 包含 Constraints
    而且 stdout 包含 Harness

  # r2/r3（harness bound 可声明口径）：fixture 需改写 config.yaml 并造带标签 .feature，
  # 保持 fast mode 文档场景；实际断言由单元/集成测试覆盖同一 CLI 边界。

  @req:r2
  场景: bdd.bindings 声明绑定源
    假如 项目在 config.yaml 的 bdd 段声明 bindings 源
    当 绑定解析器计算各 capability 的 bound 集
    那么 多源结果取并集去重且 specs 目录外的 feature 路径被忽略

  @req:r3
  场景: 未声明绑定源时输出保持现口径
    假如 项目未声明 bdd.bindings
    当 运行 llman sdd list --specs
    那么 文本保持 harness 单计数且 JSON 中 harnessBoundCount 与 harnessUnboundCount 为 null

  @req:r3
  场景: 声明绑定源后 harness 拆分为 bound 与 unbound
    假如 项目声明了至少一个 bdd.bindings 源
    当 运行 llman sdd list --specs
    那么 文本含 harness-bound 与 harness-unbound 且两者之和等于场景总数
