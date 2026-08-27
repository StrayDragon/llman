---
depends_on: []
---

合并自三个草案：sdd-human-review-flow-tuneup（流程约定）、
sdd-review-aggregate-html-view（命令化落地）、refactor-show-spec-deepen（地基深化）。
三者构成同一条「人类高效 review `.feature`」主线：深化是地基、约定是需求书、命令是落地物。
用户可见 MUST 行为新增走完整 SDD；届时新建 capability（如 `sdd-review`）+ 更新相关模板合约。

## Why

- **无固定人审入口**：unbound 场景、pending `@req`、staleness、locked-rule diff 散落在
  list/show/validate/graph 各处，每次 review 靠临场拼装命令；agents 无统一 checkpoint 指引。
- **无可视化总览**：29 个 capability 分散目录，spec 维度只有 change 侧 mermaid 图。
- **展示层是薄函数堆叠**：`show_spec` 一函数混装解析/三分支 JSON/文本渲染/错误装配，
  本期拆除 r60 双字段时被迫整块理解——review 命令将消费同一层数据，先深化再叠加。

## What Changes（垂直切片顺序即实施顺序）

- **T0 presenters 深化**：show_spec 拆 `show_spec_json(meta|full)` / `show_spec_text`
  小接口，morphology 装配收敛单一 helper；JSON schema 逐字节冻结，compat 兜底。
- **T1 人审流程落档**：AGENTS.md SDD 段新增 Human Review Checkpoint 小节
  （何时审/命令序列/分歧升级路径）；propose/verify 技能模板加面向人类的摘要段要求；
  en/zh parity 过 check-sdd-templates。
- **T2 review 聚合命令**：`llman sdd review [--capability] [--json] [--export-html <path>]`
  聚合 pending/manual rules、harness unbound、staleness、锁定 diff 提示、validate FAIL/WARNING；
  CRITICAL 存在即非零退出；数据源全部复用现成产物（morphology/staleness/req_registry/
  bindings/pageindex），零第二套解析器。
- **T3 单文件 HTML 视图**：--export-html 输出离线静态页（capability 总览表 +
  capability↔req↔scenario 层级节点图 + 过滤器），零运行时依赖、无本地 server。

## Non-goals

- LSP/编辑器插件与常驻 watch 服务缓行；crate/workspace 拆分归属 workspace-split-build-tune。
- 不做翻译文案调整之外的第二轮模板格式改造。

## Verification Sketch

- 全套 BDD 门禁 + 新 capability executable 场景（聚合数字与 list --specs 形态互恰、
  参照 r3 口径）；--export-html 产物离线浏览器可开、mermaid 可渲染；
  T0 以输出快照 diff 为空作为行为冻结证明。

## Open Questions

- HTML 模板位置：templates/sdd vs include_str! 内嵌（涉及 release 体积与检查管线）。
- review 是否吃 config 新字段（阈值/排除 capability），还是 v1 零配置。
- 锁定规则 diff 呈现粒度：仅「有变化」提示 vs base_sha 对照行。
- Human Review Checkpoint 进 AGENTS.md 自由区还是托管块（init --update 会刷新后者）。
