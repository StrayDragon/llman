---
depends_on: []
---

用户可见行为新增（新命令面），走完整 SDD；届时新建 capability 目录（如 `sdd-review`）承载
MUST/SHALL。与 `sdd-human-review-flow-tuneup` 无硬依赖，仅建议实现顺序在其后
（先有人审流程约定，再命令化落地）。

## Why

`.feature` 分散在 29 个 capability 目录，人类 review 时缺少单一聚合视图：
pending 约束、unbound `@executable` 场景、staleness、locked-rule diff、validate WARNING
各由不同命令暴露。同时「节点化总览」目前只有 `graph --format mermaid` 的 change 依赖图，
spec 维度没有任何离线可视化。

## What Changes

- `llman sdd review [--capability <id>] [--json] [--export-html <path>]`：
  - 聚合单仓 review 信号：pending/manual rules、harness unbound 场景、staleness、
    base_sha 锁定规则 diff 提示、`validate --all --strict --no-check` 的 FAIL/WARNING 清单。
  - 非零退出码策略：存在 CRITICAL 项即非零（供 CI / agent 门禁复用）。
  - `--export-html` 输出**单文件**静态页：capability 总览表 + mermaid 节点图
    （capability ↔ req ↔ scenario 层级）+ 过滤器；纯离线、零运行时依赖、无本地 server。
- 数据源全部复用现成产物：morphology JSON（r39）、staleness、req_registry（r87-r89）、
  bdd.bindings 计数（r2/r3）、pageindex tree；不发明第二套解析器。

## Non-goals

- 不做 LSP / 编辑器插件（决策过：缓行，等 review 命令形态稳定后再评估）。
- 不做常驻 watch 服务或 Web 后端。

## Open Questions

- HTML 模板放 templates/sdd 还是内嵌 include_str!?涉及 release 体积与模板检查管线选择。
- review 输出要不要吃 config.yaml 新字段（如阈值/排除 capability），还是 v1 保持零配置？
- 锁定规则 diff 的呈现粒度：仅提示「有变化」还是展示 base_sha 对照行？

## Verification Sketch

- 对当前仓库跑 `llman sdd review` 能真实命中已知 pending/unbound 数字并与
  `list --specs` 形态互恰（数字和相等，参照 r3 口径）。
- `--export-html` 产物在无网络浏览器中可独立打开且 mermaid 可渲染。
