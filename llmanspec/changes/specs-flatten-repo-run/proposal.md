---
depends_on: []
---

# 本仓库执行 specs-flatten 全库平铺（specs-flat-layout 后续）

## Why

`specs-flat-layout` change（已归档 `2026-09-11-specs-flat-layout`）交付了双布局解析与 `llman sdd project migrate --kind specs-flatten` 机制；按其 design §9-5，仓库本体不随行为变更一起迁移，另开独立 change 执行。本仓库 27 个 capability 目录全部是「纯同名单文件目录」，25 个文件带 `# scope: llmanspec/specs/<cap>` 自引用（staleness 静默失效点），正是 migrate 的目标场景；同时以真实仓库验证 migrate 实现本身。

## What Changes

- `llman sdd project migrate --kind specs-flatten --dry-run` 预检确认后执行 `--yes`：27 个目录 git mv 平铺为 `specs/<cap>.feature`，删除空目录；25 处自引用 scope 自动改写为 `llmanspec/specs/<cap>.feature`（D7）。
- 行为合约内容（@human/@executable 场景文本、锁定哈希）零变更——只动路径与 `# scope:` 头注释。
- 根 AGENTS.md「BDD 兼容测试维护规则」中 `llmanspec/specs/sdd-bdd-mode-compat/*.feature` 路径字样同步为扁平路径。
- 验证：`cargo test --features bdd`（BDD scenarios! 递归收集扁平文件）、`validate --specs --strict`、`review` 全绿后归档。
