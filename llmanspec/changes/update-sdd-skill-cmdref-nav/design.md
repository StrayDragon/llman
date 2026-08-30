# Design: update-sdd-skill-cmdref-nav

## 决策

- **D1 参考面转移**：命令参考的唯一载体 = clap help（`llman sdd <cmd> --help`）。
  agent 在 skill 内需要命令细节时 MUST 运行 --help 而非依赖内嵌表。
  skill 只保留一行指引（header/footer 也随之退役）。
- **D2 about 基线门禁保留**：cmdref.rs 瘦身为「clap help 质量门禁」——
  visible_leaves + 非空 about 单测保留；渲染 API 与 i18n 删除。
  理由：agent 直接读 help 后，about 文案是唯一命令说明面，必须有护栏。
- **D3 mermaid 恢复口径**：per-skill 导航图 = flowchart LR、当前阶段
  ★ 高亮（历史原文案恢复）；权威 TB 生命周期图仍只在 propose 渲染产物与
  根 AGENTS.md。r96 措辞从「一行文字」改回「保留导航 mermaid」。
- **D4 零兼容**：i18n `sdd.cmdref.*` 段整体删除、审计白名单删除、
  r141 整条删除（机制退役），无过渡并存。
- **D5 规模核算**：移除 ~31KB（生成块×11）+ 指引行 +0.4KB + 导航图 +4.5KB
  ≈ 净 -26KB → ~67KB。

## 测试边界

- 单测：clap about 基线（保留）；删除 cmdref 渲染相关测试；模板 parity 门禁。
- 无新 @executable（沿用既定口径）。

## 非目标

- 不动协议尾缀/Ethics/stage-guard 的 route-1 产出。
- 不做运行时元 skill。
