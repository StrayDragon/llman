---
depends_on: []
branch: fix/archive-freeze-list-flag
base_sha: 4e9e7fbb38caeccf4fe42b54e99fb67ac13f25cb
checkpointed: true
checkpoint_sha: 64529b69207f95757393a7611020a3b30c504b7b
---

## Why

`llman sdd validate` 的 INFO 提示（i18n key `proposal_depends_on_may_be_frozen` /
`proposal_blocks_may_be_frozen`）建议用户用 `llman sdd archive freeze --list`
检查被冻结的 change id，但该标志**并不存在**——`archive freeze` 只支持
`--before / --keep-recent / --dry-run / --no-interactive`。代码库只通过
`has_frozen_archive` 追踪一个布尔值（`freezed_changes.7z.archived` 是否存在），
无法枚举冻结内容；用户被迫手动 `7z l freezed_changes.7z.archived`。

来自 0.0.61 → 0.0.64 升级反馈（Crystalith 项目，BDD-off，50 specs）：升级路径
顺畅，唯一卡点即此提示与实际命令不一致。

## What Changes

- `llman sdd archive freeze` 新增 `--list` 布尔标志：枚举冷备份归档
  `freezed_changes.7z.archived` 内已冻结的 change 目录（顶层 `YYYY-MM-DD-id`）。
- `--list` 优先短路：列出时不执行任何冻结/删除/写入操作（只读）。
- 复用 thaw 的 `sevenz_rust2::decompress_file` → tempdir 模式，枚举顶层目录并
  排序输出。无归档文件时打印 "No freeze archive found"；空归档时明确提示。
- 使既有的 validate INFO 提示变为** truthful**（无需改动 locale 文案）。

## Capabilities

- `sdd-workflow`（新增 requirement r92，承载 `archive freeze --list` 的
  MUST/SHALL 行为合约；可执行场景写入既有
  `archive-freeze-and-gates.feature`）。

## Impact

- **用户可见**：新增只读子标志；不改变 freeze/thaw 现有语义。
- **校验**：既有 validate 提示由"误导"变为"可用"。
- **测试**：新增 3 个集成测试覆盖（有内容 / 无归档文件 / 不改文件系统）。
- **向后兼容**：纯加法，无破坏性。