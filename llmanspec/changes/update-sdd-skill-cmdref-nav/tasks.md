# Tasks: update-sdd-skill-cmdref-nav

- T1 模板：变量换指引 + 导航 mermaid 回归
  28 个模板 `{{ sdd_command_reference }}` → 一行指引（zh/en）；从 `e329cf4^` 恢复 10 个 skill 的 per-skill 导航 mermaid（propose 仅恢复 Skill 导航 LR 图，TB 权威图未动过）；resync。
- T2 代码：渲染链路退役 + help 质量门禁保留
  删 templates.rs 注入、cmdref.rs 渲染 API 与 i18n 渲染测试；保留 visible_leaves + about 非空门禁；删 locales `sdd.cmdref.*` 与审计白名单；单测全绿。
  [blocked-by: T1]
- T3 specs landing
  修订 r139（命令参考=CLI help、skill 禁内嵌命令表、about 基线 MUST）、删除 r141、再修订 r96（导航 mermaid 恢复）。
- T4 全量门禁
  fmt/clippy/测试/模板门禁/i18n 审计/validate --all/review；量测渲染产物总体积（预期 ~67KB）；建议 llman-sdd-verify。
  [blocked-by: T1, T2, T3]
