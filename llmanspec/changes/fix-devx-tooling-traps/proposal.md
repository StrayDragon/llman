---
depends_on: []
---

合并自两个独立草案（fix-tasks-checkbox-parse-trap + fix-i18n-locale-stale-rebuild），
减少散落：两者同为工具链小正确性修复、同源于 toon-longtail-consistency-purge 的
apply/verify 循环实录、均倾向快速路径。实现时按 T1→T2 两切片可拆成相邻小 PR。

## Why（两条实录）

1. **勾选框解析陷阱**：`src/sdd/shared/tasks.rs::is_checkbox_line` 把任何 `- [` 前缀行
   当勾选框，内容非空即 Pending；r101 允许的 `[blocked-by]` 依赖标记按直觉写在行首
   （如 `- [blocked-by: T1]`）会被当成未完成任务，`validate --strict` 直接 ERROR——
   提案阶段干净工件反而过不了门禁（b306322 被迫返工）。归档惯例的绕法
   （依赖写成任务行内尾缀）无文档约束且语义错位。
2. **i18n 陈旧重编陷阱**：`i18n!("locales")` 过程宏嵌入翻译串但不追踪 locale 文件变更，
   改 `locales/app.yml` 后测试二进制仍携带旧文案；apply R2 出现「next-step 丢失 live
   关键词」的假回归，被迫 `touch src/lib.rs` 强制重编（commit 4080890）。
   静默陈旧对 CI 与本地同等危险。

## What Changes

- **T1 解析收窄**：仅 `[ ]` / `[x]` / `[X]`（含前导空格变体）识别为勾选框；
  其余方括号内容行视为普通文本，不进 completion 统计。extract_task_text 同步适配，
  既有 `- [x]/- [ ]` 用例行为不变。
- **T2 locale 重编追踪**：build.rs（或等价机制）对 `locales/**` 声明
  `cargo:rerun-if-changed`；若 rust-i18n 已有内建开关则升级启用替代自写。
  AGENTS.md 测试节补一句备忘：改翻译无需手工 touch。

## Non-goals

- 不引入 markdown 任务列表标准以外的语法；不改 completion_ratio 对外语义。
- 不迁移到运行时翻译加载（保持编译期内嵌、二进制自包含）；不动翻译格式/schema。

## Verification Sketch

- T1 单测：`- [blocked-by: x]` 不计数；`[x]/[X]/[ ]` 行为不变；含独立 blocked-by 行的
  tasks.md 过 `validate --strict`。
- T2 配方：改一个 value → 不 touch 直接跑 compat 测试断言新文案出现；
  记录增量时间开销（预期毫秒级）。

## Open Questions

- blocked-by 引用悬空（不存在的 task id）要不要 WARNING？可与 check_dag_cycles 联动评估。
- rust-i18n 4.x upstream 是否已内建修复（若是，T2 降级为仅补 AGENTS.md 备忘）。
