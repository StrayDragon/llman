---
depends_on: [git-native-v2]
---

# 生命周期自表达：draft 落地足够信息，配置趋零，cli 开箱即用

## Why

现行生命周期要求使用者预知并手工维护多类「元配置」：proposal frontmatter 的 `skip_specs_landing`、`rules_edit_acked`（v2 后为 `rules_touched`）、stage 三态的推进命令（change start 前 must designed）。这类配置的本质是**让工具在事后能推断意图**——但意图在事件发生时就已存在，理应由工具在事件时刻捕获，而不是让使用者预先声明。需求方方向（2026-09-11 探索拍板）：把足够的信息在 draft 阶段就落地保存；尽可能减少 `rules_edit_acked` 类配置；cli 开箱即用。

## What Changes（方向草案，正式化时细化）

1. **draft 即自足**：`change new --from` 在创建时就把「为什么 / 影响哪些 capability / 是否涉合约」的问答固化为 proposal 内容（交互式或由描述推导），后续 stage 推进不再依赖文件齐全度推断。
2. **配置趋零**：`skip_specs_landing` 由 landing 判定自然导出（diff 为空即视为无合约变更）；`rules_touched` 改为编辑时点交互确认或 finalize 时从实际 diff 推导待确认项，取代预声明。
3. **cli 开箱即用**：`change start` 遇脏树自动打 WIP commit（计划壳本就要进分支）；`finalize` 自动 commit（生成规范 message，`--amend` 可改）；`checkpoint` 退役（finalize 已吸收）；draft/designed 两态合并为单一 planning 态。

## Capabilities / Impact

- `sdd-workflow`（r50 三态、r98 start 门禁、r42 finalize 语义、checkpoint 相关条文）
- `cli-experience`（交互流）

> 依赖 `git-native-v2` 先行（范围换锚与 rules_touched 落地后再减配置，避免同时动两套语义）。当前为方向草案；正式化走 propose。
