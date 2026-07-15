# language: zh-CN
# 对应 spec: sdd-workflow — BDD-on solidify 工作流：
# spec.toon = SSOT, .feature = 可执行子集, feature 字段显式控制, 框架无关
功能: BDD-on solidify 工作流
  场景: spec.toon 恢复完整结构含 feature 字段
    假如 config.yaml 含 bdd: 段
    当 用户编辑 specs/<id>/spec.toon
    那么 spec.toon 含 kind/name/purpose/valid_scope/requirements/scenarios
    而且 scenario 含 feature 布尔字段（默认 true）
    而且 valid_scope 必填、requirements 必含 MUST 或 SHALL

  场景: solidify 按 feature 字段写入 feature
    假如 delta op_scenarios 含 CLI 可执行步骤且 feature 未显式设为 false
    当 用户运行 llman sdd solidify <change-id>
    那么可执行场景写入目标 specs/<id>/<id>.feature
    而且 feature=false 的场景不写入

  场景: solidify 跳过自指递归场景
    假如 op_scenario 的 when 含 llman sdd validate
    当 用户运行 llman sdd solidify <change-id>
    那么该场景不写入 .feature

  场景: solidify 框架无关不扫描 step binding
    假如 BDD run_command 为任何框架
    当 user calls llman sdd solidify <change-id>
    那么 solidify 不读取 bdd_steps.rs 或任何框架特定文件
    而且仅基于 feature 字段 + 自指黑名单做过滤

  场景: archive 不再复制 feature 文件
    假如 config.yaml 含 bdd: 段
    当 用户运行 llman sdd archive run <change-id>
    那么 archive 不处理 .feature 文件

  场景: propose 只产生 TOON delta 不引导 feature
    假如 config.yaml 含 bdd: 段
    当 agent 执行 llman-sdd-propose
    那么 delta 仅含 TOON spec.toon

  场景: solidify-migrate 升级现有 spec
    当 用户运行 llman sdd project solidify-migrate
    那么所有 BDD-on spec.toon 升级为完整结构
    而且 .feature 内容反向填充到 scenarios 表（全部 feature=true）
