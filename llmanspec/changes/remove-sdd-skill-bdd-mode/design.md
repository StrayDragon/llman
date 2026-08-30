# Design: remove-sdd-skill-bdd-mode

## 决策

- **D1 只退元数据不退 runner**：`bdd:` 段（run_command/bindings）是
  `validate --check` 的 GWT 执行引信，与「管线模式」无关；退役它属于另一个
  量级的语义变更（用户确认保留）。
- **D2 skill_set 保留**：default/optional 枚举仍被 r90 候选集清理与 optional
  渲染逻辑使用；`metadata.llman_sdd` 块整体保留，只少 bdd_mode 一行。
- **D3 门禁机器去向**：r95 的「缺失 llman_sdd / skill_set 非法 → 非零退出」
  门禁保留（防手写 skill 绕过托管协议）；删除的只有 bdd_mode 的解析、期望值
  计算与一致性比对。
- **D4 零兼容**：旧渲染产物中的 bdd_mode 行由 init --update 重写消失；
  校验路径不再读取该键（旧产物残留键不被报错——validate 只查门禁所需键）。

## 测试边界

- 单测：skill_consistency 元数据门禁（缺失/非法 skill_set）；
  bdd_steps/it 中 bdd_mode 断言删除或改写；既有 r95 场景经新 fixture 继续驱动。
