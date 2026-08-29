# Design: apply 阶段 Commit 纪律指引

## 决策

### D1: 模板指引，不立合约规则

commit 纪律以 skill 模板内容承载（`skip_specs_landing: true`），不在
`llmanspec/specs/**` 新增 MUST 规则。理由：它是给 agent 的工作方式指引，
不是 CLI 外部可观测行为；先以模板落地观察效果，若后续需要 verify 可检查
的强制性（如「apply 模板必须含 Commit 策略节」），再以独立 change 补
`sdd-structured-skill-prompts` 规则。

### D2: 指引落点与措辞强度

放在 apply skill 模板「硬约束」之后、步骤之前的显式小节（en: Commit
Policy / zh: Commit 策略），用 MUST/SHOULD 措辞约束 agent 行为：

- 循环内逐 task commit → MUST NOT（治 T1–T14 式步骤日志）。
- checkbox 勾选 → 只改工作区（治「勾一个提交一次」）。
- 收尾默认 `change finalize` 单 commit → SHOULD（与 archive skill 现有
  「推荐单 commit 收尾」呼应，不新增机制）。
- blocker 中断 → 一次性 WIP commit 快照（保留现场，避免丢工作）。

### D3: 双 locale 对等

en 与 zh-Hans 两棵模板树同步新增该节，语义对等、措辞各自地道；模板版本
头按 `check-sdd-templates` 要求同步 bump，避免 parity 门禁失败。

## 权衡

- 「允许 dirty tree 到 verify 之后」与既有 `change start` 干净树门禁不冲突：
  start 只在 propose→apply 交界检查一次，apply 期不再有 CLI 级干净树检查
  （finalize 本身不要求干净树）。
- 不改 `change start` 的干净树门禁来合并规划壳 commit：那是 r102 的刻意
  安全设计，放宽属更大的合约手术，收益仅省 1 commit/变更。
