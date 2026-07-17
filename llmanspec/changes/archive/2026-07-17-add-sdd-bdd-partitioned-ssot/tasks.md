# Tasks: add-sdd-bdd-partitioned-ssot

## 1. 合约与解析

- [x] 1.1 实现 `@req` / feature_delta TOON 解析（scenario 块级）
- [x] 1.2 validate：链接完整性、双写检测、不可执行 id 入侵 feature
- [x] 1.3 更新 `sdd-bdd-mode-compat` 测试与 `.feature` 场景（solidify / validate / index）

## 2. CLI 形态与 solidify / archive

- [x] 2.1 `list --specs` / `show` 人读分段 + JSON `morphology`
- [x] 2.2 solidify：一致性门禁 + 可选 `--write-stubs`；BDD-off no-op 保留
- [x] 2.3 feature_delta apply + archive 双管道；移除整文件 feature 覆盖复制
- [x] 2.4 `sdd project partition-migrate [--dry-run]`

## 3. context / index

- [x] 3.1 rebuild：可执行 GWT 仅来自 feature；toon 仅非可执行；碰撞 feature 胜且不双 embed
- [x] 3.2 更新 `sdd-context` 相关单测 / compat `index-embed`

## 4. Skills 与文档

- [x] 4.1 更新 propose / solidify / archive / apply / verify / explore / graph / compact 技能 + `AGENTS.md` 为 Partitioned
- [x] 4.2 下游迁移说明：`MIGRATION.md`

## 5. Dogfood 本仓

- [x] 5.1 对全部 `llmanspec/specs/*` 跑 `partition-migrate`
- [x] 5.2 `llman sdd validate --specs --strict --no-check` 绿（30/30）
- [x] 5.3 `cargo test --features bdd` 绿
- [x] 5.4 `just check` 绿

## 6. 收尾

- [x] 6.1 `llman sdd validate add-sdd-bdd-partitioned-ssot --strict --no-interactive`（tasks 全勾后）
- [x] 6.2 apply 闭环完成 → 交 `llman-sdd-verify`（本项不在 apply 内跑 verify 报告）
- [x] 6.3 WARNING 修复：r5/r6/r8 harness（`feature.delta` + BDD Given/bindings）+ 发布 `RELEASE_NOTES.md` / `UPGRADE_AGENT_PROMPT.md`
