# Tasks: add-sdd-cmdref-generation

> 收尾单 commit 由 finalize 负责；测试口径：仅 Rust 单测 + 既有模板门禁。

- [x] T1 clap 命令树遍历器 + 双语 one-liner
  `crates/llman-sdd`：新增 cmdref 注册表模块（walk `SddCommands` clap Command，跳过 hide；i18n `sdd.cmdref.*` en+zh-Hans one-liner，缺 key 回退 clap about）；补齐薄弱子命令的 clap doc comment；单测覆盖注册表完整性/回退/无已删命令（spec-md2toon 回归样本）。
- [x] T2 渲染注入 + 静态单元退役
  `build_template_vars` 注入 `sdd_command_reference`（按 locale）；`templates/sdd/{en,zh-Hans}` 全部模板替换 include 为变量；删除 `units/skills/sdd-commands.md`（双 locale）；单测：渲染产物含生成块、双语正确；`init --update` resync + `check-sdd-templates` 绿。
  [blocked-by: T1]
- [x] T3 协议尾缀重写 + Ethics 一行化
  重写 `units/skills/structured-protocol.md` 与 `units/skills/ethics-governance.md`（双 locale）：六节保留、正文 3–5 行自检、Ethics 每模板一行具体声明；渲染产物 resync。
- [x] T4 导航 mermaid 换文字 + stage-guard 收窄
  除 propose 外全部 skill 的导航 mermaid → 一行位置文字；权威图保留 propose；stage-guard 表仅留 apply/verify，explore/apply-cycle 一行化；resync + 模板门禁。
  [blocked-by: T2]
- [x] T5 propose 非阻塞 id 推导
  propose 模板硬约束改为「用户给出 id 则用之，否则按 r99 规则推导并宣布后继续」；清扫其余 skill 中「确认 id」类阻塞措辞；resync。
- [x] T6 陈旧语义清扫
  全库 grep：spec-md2toon 残留（模板/渲染产物/文档）、propose 生命周期图「toon / feature」叙述改单轨；`just qa` i18n 死 key 审计过。
  [blocked-by: T2]
- [x] T7 全量门禁与收尾
  单测全绿 + `just check` + `check-sdd-templates` + `validate --all --strict` + `review` critical=0；建议 `llman-sdd-verify`。
  [blocked-by: T1, T2, T3, T4, T5, T6]
