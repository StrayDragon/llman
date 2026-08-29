# Design: 移除 `llman sdd status`

## 决策

### D1: 直接删除，不走 deprecate 过渡

`llman sdd status` 是开发者工具的内部命令面，无第三方脚本生态负担；
保留 deprecated 别名只会延长双查询路径并存的认知成本。替代路径
（`show <id> --type change --output json`、`list --json`）字段均为超集：
stage / specsLanded / skipSpecsLanding / readyToImplement 由 show 承载，
枚举、计数、morphology 由 list 承载。

### D2: `list --json` 自身的 `status` 字段不动

`tests/it/sdd_integration.rs` 断言的 `change["status"] == "in-progress"`
是 change 生命周期态字段（list 输出 schema），与被删命令同名不同物。
本次只删命令，不改 list 输出 schema。

### D3: 合约面改动范围

- `cli.feature`：删 r42（status TOON 输出与 target 解析）。r42 无
  `@executable` 验收（该 spec 仅有的两个验收场景挂在 r112 前缀匹配上），
  删除无 orphan 连带。`# purpose:` 头注释同步去 status 表述。
- `sdd-workflow.feature`：r1 / r93 / r126 仅做措辞收敛（去掉 status
  查询面表述），规则语义不变——三条均为 `@human`，已获
  `rules_edit_acked: true`（lock-gate 以 INFO 记录）。
- `sdd-bdd-mode-compat.feature` 不动：全文无 status 字样，smoke 列表
  是实现层细节，缩列不需合约变更。

### D4: i18n 清理边界

只删 status 命令专属 key 段（no_changes_dir / no_active_changes /
no_tasks_status / complete_status / just_now / changes_header / specs_header /
no_specs 等，以 status 命令渲染路径实际引用为准）；`editor_exit_status`、
`staleness_status` 等同名无关 key 保留。嵌入通用错误文案的
`run llman sdd status` 引导改为 `llman sdd show` / `llman sdd list`。

### D5: bdd_steps fixture 政策

`tests/bdd_steps.rs` 中 spec.toon 迁移 fixture 的
`"run llman sdd status"` 是模拟历史遗留文件的**数据**，非行为耦合；
apply 时验证该 fixture 驱动的测试不执行此字符串后保留原样，
避免无意义的历史数据改写。

## 权衡

- 前缀匹配定位 change 的入口随 status 消失：r112 已把前缀匹配推广到
  show/validate/graph/change 全命令面，status 的独有前缀能力无增量价值。
- `c<N>-` 优先级排序展示丢失：仅 status 渲染使用，list 按 id 排序已够用；
  若后续需要可在 list 加排序 flag（新 change），不在本 change 范围。
