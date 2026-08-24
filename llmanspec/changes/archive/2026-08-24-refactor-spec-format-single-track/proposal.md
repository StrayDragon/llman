---
depends_on: []
branch: sdd/refactor-spec-format-single-track
base_sha: be851c07c4ef5fa961d0fe82f0f7a5fdc7aa9309
checkpointed: true
checkpoint_sha: be851c07c4ef5fa961d0fe82f0f7a5fdc7aa9309
---

## Why

当前规格体系是 Partitioned SSOT 双轨：`spec.toon`（TOON 表格：元数据 + requirements 散文 + 不可执行 note 场景）+ `*.feature`（可执行 GWT）。实践暴露三个结构性问题：

1. **约束内容无法程序化约束**：requirement 是自由散文，validate 只能查结构（链接/双写/Gherkin 合法性），查不出重复、歧义、空洞；数量增长后人类无法 review 行为正确性。
2. **AI 阅读易被误导**：同一行为的事实分散在两个文件，长 prose 嵌在 TOON 表格中，agent 收集上下文时语义密度低且无「此处勿动」信号。
3. **防御性门禁复杂**：dual-write 双写检测、Partitioned 权威划分等门禁机器（validation.rs 约 2300 行）防御的正是双轨格式自身造成的问题。

探索结论（含参考项目 xylitol / crystalith / scalim 调研与 rstest-bdd 0.6.0-beta3 源码验证）：单轨 feature-as-spec 可行——`scenarios!` 宏只展开顶层场景（`Rule:` 块会被静默跳过），但 tag 是安全且已验证的机制；`@executable`/`@req:` 标签链路已在本仓跑通。

## What Changes

- **格式单轨化**：删除 `spec.toon` 格式与全部 TOON 解析/序列化（`toon-format` 依赖退役）；每个 capability 的唯一规格载体为单个 `.feature` 文件：
  - 头注释承载元数据：`# capability:` / `# purpose:` / `# scope:`（staleness/context 继续消费）；
  - `@req:<id> @human` 场景承载约束层（statement 全文放场景 description，无损迁移；GWT 槽位鼓励性分解）；
  - `@req:<id> @executable` 场景承载可执行验收层（现有 88 个 feature 内容零迁移）。
- **约束强制三态分级**（新 morphology，取代 harness bound/unbound 二分）：
  - `rule_enforced`：约束有 ≥1 个 `@executable` 场景实现（自动化兜底）；
  - `rule_manual`：显式 `@manual` 豁免（人审）；
  - `rule_pending`：两者皆无（覆盖缺口）。
- **锁定门禁**：所有 `@human` 场景计算规范化哈希（id+name+description+steps）；change 门禁比对 `base_sha...HEAD`，哈希集合增删改 → ERROR，除非 proposal frontmatter 带 `rules_edit_acked: true`（人工确认解锁）。
- **validate 门禁换血**：删 dual-write / Partitioned 权威 / BDD-off 分叉语义；增锁定完整性、`@human` 归一化查重、tag 语法校验、孤儿 acceptance WARNING。`bdd:` 段彻底退化为纯 runner 开关。
- **子命令适配**：`list --specs`/`show`（新三态计数 + 覆盖矩阵）、`index rebuild`（两类场景均带 req_id 入树）、`context`（携带分级标记）、`change diff/finalize/checkpoint`（specsLanding glob 收窄为 `*.feature` + 锁定检查）、`spec scaffold`（产出单文件骨架）、`resolve-req`/`next-req-id`（注册表改为跨 feature 扫描 `@req`）。
- **一次性迁移**：新增 `project migrate --kind toon2features`（requirements[] → `@human` 锁定场景；note 场景丢弃；同目录 feature 合并；幂等）。本 change 内同步完成 llman 自身 28 个 spec 的自迁移（大爆炸，finalize 前全绿）。
- **模板与 skills 改版**：`templates/sdd/**`、`.agents/skills/llman-sdd-*`、根/llmanspec AGENTS.md 托管内容同步到单轨叙事；提供下游仓库（xylitol 等）迁移 prompt。

## Capabilities

- 新增 `spec-format`：单轨 feature-as-spec 格式契约（tag 语法学、头注释元数据、锁定哈希规范化、三态 morphology 定义）。
- 重写 `sdd-bdd-mode-compat`：收缩为 runner 开关兼容契约（Partitioned r5/r6、BDD-on/off 生命周期分叉合约废除）。
- 更新 `sdd-workflow`：specsLanding 路径口径、锁定门禁在生命周期中的挂点。

## Impact

- **破坏性**：`spec.toon` 不再被读取；`migrate --kind spec-md2toon` 与 partitioned 相关命令/门禁移除（沿用 solidify 零兼容先例，validate 遇遗留 spec.toon 报 ERROR 并指向 toon2features）。
- 下游 BDD-off 项目（scalim/crystalith 形态）须跑一次迁移；fast mode 结构校验对其继续可用（runner 开关语义不变）。
- 术语债：「harness bound/unbound」措辞退役，文档与输出逐步切换为 rule 三态口径。
