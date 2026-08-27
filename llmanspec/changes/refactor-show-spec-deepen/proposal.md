---
depends_on: []
---

arch-review 深化候选（探索期即编号：verify 建议项 #1）。内部重构，倾向快速路径；
若拆解中发现 morph 口径被行为合约钉死而必须动 `@executable`，再升级 propose。

## Why

`src/sdd/shared/show.rs::show_spec` 一函数承载：spec 文件解析、morphology 计算、
JSON 三分支（meta_only / requirements / 全量）、人读文本渲染与错误装配。
属于「接口 ≈ 实现」的薄函数堆叠——调用者没省事，改动时常牵一发动全身
（本期 r60 双字段拆除时被迫整块理解该函数）。

## What Changes

- 按展示面拆分 presenter：`show_spec_json(meta|full)` 与 `show_spec_text`
  各自成为小接口；共享的 morphology 装配收敛到单一 helper。
- JSON 输出 schema 逐字节不变（morphology / requirements 键集冻结），
  由现有 compat 测试兜底回归。

## Non-goals

- 不改任何用户可见输出（含错误消息文案）。
- 不引入新的 serde 类型层（Primitive Obsession 防反弹）。

## Verification Sketch

- `cargo test -q --test sdd_bdd_compat_tests` + lib show 相关单测全绿。
- `llman sdd show <spec> [--json|--meta-only]` 输出与重构前快照 diff 为空。

## Open Questions

- list --specs 与 show 的行/段渲染是否存在可合并的重复逻辑（Duplicated Code）？
