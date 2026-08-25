# Design: fix-toon2features-preserve-features

## 职责边界（核心决策）

迁移是**格式搬运**，不是**内容合并器**。旧世界两类资产职责不同：

| 旧资产 | 性质 | 迁移处理 |
|--------|------|---------|
| `spec.toon` requirements[] | 约束（statement） | 转为 `@req:<id> @human`（statement 原文入描述）——不变 |
| `spec.toon` scenarios[] 行 | 文档性注记（含 GWT 分解） | 含 GWT 且配对 → `@req:<id> @human` 场景 + 步骤（语言自适应）；不配对/无内容 → 记账丢弃 |
| 目录中既有 `*.feature` | **活 harness 资产**（rstest-bdd 按 tag 直驱） | **一律不动**（不读、不写、不删）；报告计数 `left`，r131 合并交人工 |

动机：机器合并正是 v0.0.67 丢 39 个 @executable、需 0a097d5 + 5367d83 两轮召回的根因；合并涉及步骤文本可驱动性判断（rstest-bdd 按精确文本绑定），只能人工审。

## 语言检测链（每项都有测试）

1. config `bdd.default_language`（显式 Gherkin 配置，最高优先）
2. config `locale` → `locale_to_gherkin_lang`（zh-Hans → zh-CN）
3. `llmanspec/specs/**` 中任一 `.feature` 的 `# language:` 头（用户所说「随机找一个已有的 features」）
4. 兜底 `en`（Given/When/Then）

渲染：新增 `dump_main_spec_lang(doc, lang)`（`dump_main_spec` 保持 zh-CN 默认并委托，interop 不变）；迁移产物统一写 `# language: {lang}` 头。单元格遗留关键字前缀（英文 Given/When/Then/And/But 与中文 假如/当/那么/而且/并且/但是，后随空白）剥离后按目标语言重加前缀。

## skip 策略（主 .feature 已存在）

a275d6a 的 spec-format 目录同时有 `spec-format.feature` + `spec.toon`（真实案例）。选择：跳过 + 警告「人工合并后重跑」，保留 toon 零丢失。否决「追加进主文件」（违反不动原则）与「写旁路文件」（加剧 r131 多文件错误）。

## 诚实中间态

迁移后目录含主 `.feature` + 遗留多文件 `.feature` → 单轨 `validate` 如实报 r131「merge them」错误；BDD harness 不受影响（tag 驱动，扫全部 `.feature`）。报告输出 `left N legacy .feature file(s) untouched — merge per r131` 明示后续动作。

## 备选与否决

- 保留 feature=true → @executable 转写：与「不动 .feature」组合会产生双份验收（toon 转写 + 原 .feature），且 toon 行文本未必可驱动；废弃（上一 change 的该路径被本 change 取代）。
- 检测链把 `.feature` 头置于 config 之前：config 是显式意图，嗅探是启发式，config 优先更可预期。

## 风险

- r136 为锁定规则，改写需 `rules_edit_acked: true`（已在 frontmatter）。
- 旧 BDD fixture `已初始化含遗留 spec.toon`（toon 与主 feature 同目录）语义从「可迁移」变「skip」——正好复用为 skip 场景 fixture。
