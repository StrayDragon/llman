---
depends_on: []
rules_edit_acked: true
---

# skill 命令参考改由 agent 查 CLI help + 导航 mermaid 回归

## Why

`add-sdd-cmdref-generation` 把命令表改为渲染期生成后，实测净收益只有 -5.3%
（98.9KB→93.7KB）：生成块虽不再漂移，但 29 条命令 × 11 个 skill 的嵌入成本
（~31KB）吃掉了瘦身大头。用户复盘后决策：

1. **命令参考彻底不进 skill**：agent 需要命令细节时自己运行
   `llman sdd <cmd> --help`——CLI 即参考，零维护、零 token 成本、永不陈旧。
2. **per-skill 导航 mermaid 回归**：可视化「当前阶段→下一步」值得每 skill
   ~450B 的成本，恢复 T4 移除的导航图。

净效果预估：93.7KB → ~67KB（对 route-1 前 99KB 约 **-32%**）。

## What Changes

- 模板移除 `{{ sdd_command_reference }}`，替换为一行指引
  （「命令细节用 `llman sdd <cmd> --help`；命令参考以 CLI 为准」）。
- 恢复 10 个 pipeline/辅助 skill 的 per-skill 导航 mermaid（从
  `e329cf4^` 历史恢复原文案），propose 的权威 TB 生命周期图保持不变。
- 删除渲染链路的生成注入与 cmdref 渲染 API（`sdd_command_reference` /
  one_liner / i18n `sdd.cmdref.*` 段 / i18n 审计白名单）。
- **保留并重定位** clap about 基线门禁：`visible_leaves` +
  「所有可见叶命令 MUST 有非空 doc comment」单测——命令参考退到 `--help`
  后，help 文案本身成为 agent 直接阅读面，基线门禁更重要。
- specs：修订 r139（命令参考=MCLI help，skill MUST NOT 内嵌命令表）、
  删除 r141（生成式变量机制随之退役）、再修订 r96（导航 mermaid 恢复为
  MUST；权威图口径不变）——三条均为锁定规则，ack 已带。

## Capabilities

- `sdd-structured-skill-prompts`：r139 修订、r96 再修订（nav mermaid 回归）。
- `sdd-template-units-and-jinja`：r141 删除。

## Impact

- 受影响范围：28 个模板（双 locale）、渲染产物 resync、`cmdref.rs` 瘦身为
  help 质量门禁、`templates.rs` 注入点移除、locales 删 `sdd.cmdref.*`、
  `check-i18n-keys.py` 白名单移除、3 条 live 规则修订。
- 行为变化：skill 正文 -~27KB（净 -32% vs route-1 前）；agent 获取命令细节
  的路径变为运行时 `--help`（离线可用，同一二进制）。
- 测试口径：单测 + 模板门禁（沿用既定偏好，无新 @executable）。
- 不做：动态元 skill（仍缓行）；任何形式的命令表回嵌。
