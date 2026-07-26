# SDD Pipeline — BDD-off（已废弃）

> **Superseded.** 本项目已统一为 Git-native 单轨流程，不再区分 BDD-on / BDD-off 生命周期。
>
> 请参阅：
> - [docs/sdd/README.md](./README.md) — 统一流程概览
> - [docs/sdd/pipeline-bdd-on.md](./pipeline-bdd-on.md) — Git-native 闭环（现适用于所有项目）

## 历史说明（仅供迁移参考）

旧 BDD-off 流程曾在 `changes/<id>/specs/` 下编写 TOON delta，由 `change archive` 合并进主 `spec.toon`。该路径已于统一生命周期变更中**移除**：

- `change delta` 子命令 → 拒绝（编辑 live specs）
- archive TOON delta merge → 替换为 ff-merge + docs rename
- `specified` 阶段 → 合并为 Draft / Designed / Full 三态

`bdd:` 段保留为 **runner-only** 开关，不影响变更阶段判定。
