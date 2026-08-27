---
depends_on: []
---

本草案将走完整 SDD 流程（Branch binding → Specs landing）：必须编辑 live
`llmanspec/specs/**/*.feature` 文本。注意 sdd-workflow / spec-format 中多数待改行是
`@human` 锁定场景 —— 修改前须人工 ack 并在 frontmatter 写入 `rules_edit_acked: true`。

## Why

spec.toon 废除（spec-format r130：live 口径收窄为 `llmanspec/specs/**/*.feature`）后存在三类长尾残留：

1. **live `.feature` 文本仍把 spec.toon 当现行合约载体描述**（21 处命中：
   spec-format×9、sdd-workflow×8、sdd-structured-skill-prompts×2、sdd-context×1、
   sdd-bdd-mode-compat×1）。已知确凿过期样本：sdd-workflow r60（Constraints/Harness
   分段引用 spec.toon 来源）、r61（「约束与不可执行场景编辑 spec.toon」）、r107（回写更新
   spec.toon statement）、r114（scaffold 要生成 spec.toon + 可选 .feature）。
   这与 AGENTS.md「每个 capability 只有一个 `<capability>.feature`；spec.toon 出现即
   ERROR」直接矛盾。
2. **src 12 个文件**引用 `SPEC_FILE = "spec.toon"`；discovery 注释自述
   "Legacy spec.toon still counts so the resolver can …"。其中 migrate md2toon、
   出现即 ERROR 的检测属合法保留，但疑似仍有双载体解析支持混入 —— 需逐文件审计分类。
3. **templates/sdd 14 个文件**提及；`units/spec/feature-contract.md` 已是新口径
   （"It is the only spec artifact — there is no spec.toon."），但
   propose/verify/archive 等 SKILL 模板与 validation-hints 未同步。locales/app.yml 亦有字样。

合约文本脏会让下游（review 可视化、morphology、pageindex 摘要）把过期叙述当现行语义读出。

## What Changes

- 逐处分类 live `.feature` 中的 spec.toon 提及：改为单载体口径措辞 or 保留显式历史注记
  （如 migrate / ERROR-gate 语境），产出审计表落到 design.md。
- sdd-workflow r60「show 分段」在单载体世界的语义重述（或降级删除——见 Open Questions）；
  r114 scaffold 文案与实现对齐（仅生成 `<capability>.feature` 骨架）。
- src 侧收敛：非法残留按「出现即 ERROR」检测保留；多余的双载体兼容解析路径移除。
  migration 面仅保留 `project migrate --kind spec-md2toon`。
- templates/sdd 全量同步至单载体口径，en / zh-Hans parity 通过 `just check-sdd-templates`。

## Non-goals

- 不引入任何新的 spec 载体格式。
- 不改变三态 stage、Specs landing、lock gate 的行为语义。

## Open Questions

- r60（show 分段展示 Constraints 与 Harness）在 spec.toon 废除后，「来自 spec.toon 的
  requirements」这一半已无对象：分段语义整体重述为 `@human/@executable` 两类场景摘要，
  还是整条 requirement 删除？涉及锁定规则改动，须先走 ack 流程。
- discovery 的 legacy resolver 宽容度：命中 spec.toon 时从"仍计入解析"改为"直接报 ERROR +
  指引 toon2features"，还是维持现状？影响 sdd-bdd-mode-compat 既有断言。

## Verification Sketch

- `grep -rn 'spec\.toon' llmanspec/specs/` 仅剩带历史语境的条目（每处须有存在理由注释）。
- `llman sdd validate --all --strict --no-check` 与 full mode 全绿。
- `just check-sdd-templates` 绿。
