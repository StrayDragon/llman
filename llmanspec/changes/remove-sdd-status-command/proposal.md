---
depends_on: []
rules_edit_acked: true
branch: sdd/remove-sdd-status-command
base_sha: 6514cf628231ac01bc255b1060cd81348c257df2
checkpointed: false
---

# 移除 `llman sdd status` 子命令

## Why

`llman sdd status` 与既有命令面冗余：stage / specsLanded / skipSpecsLanding /
readyToImplement 等 apply 门禁字段已由 `llman sdd show <id> --json --type change`
全覆盖，枚举与计数由 `llman sdd list --json` 全覆盖。status 独有的只有聚合计数
与 `c<N>-` 前缀排序展示，信息价值低；同时它是 agent 指引文档中的第二查询入口，
增加 skill 模板与 AGENTS.md 的认知负担。移除后查询路径收敛到 show/list 单一路径。

## What Changes

- 删除 `llman sdd status` 子命令：`crates/llman-sdd/src/sdd/commands/status.rs`
  及 `command.rs` 的枚举变体与分发（约 243 行、830-834 行）。
- 删除 `src/bin/llmanspec.rs` 中 `llmanspec status` 的别名映射。
- 清理 `locales/app.yml` 中 status 命令专属 i18n key 段
  （no_changes_dir/no_active_changes/no_tasks_status/complete_status/just_now 等），
  并改写 frontmatter 未知字段错误提示中嵌入的 `run llman sdd status` 引导为
  `llman sdd show` / `llman sdd list`。
- 合约收紧（Specs landing，`rules_edit_acked: true`，用户已确认）：
  - `cli.feature`：删除 r42（status 命令 TOON 输出与 target 解析）整条 @human
    规则；更新 `# purpose:` 头注释去掉 status 表述。r42 无 @executable 验收，
    删除不产生 orphan。
  - `sdd-workflow.feature`：改写 r1 / r93 / r126 三条规则中的 status 措辞
    （查询面收敛为 show/list；stage 推断同源面去掉 status）。
- 文档同步：`llmanspec/AGENTS.md` 两处、根 `AGENTS.md` 一处的
  `llman sdd status` 查询指引改为 show/list。
- 模板同步：`templates/sdd/{en,zh-Hans}/skills/{draft,propose,apply-cycle}.md`
  移除 status 引用，`llman sdd init --update` resync 渲染产物，过
  `just check-sdd-templates`。
- 测试同步：`tests/it/sdd_bdd_compat.rs:197` smoke 列表移除 `["sdd", "status"]`；
  `tests/bdd_steps.rs:487` 的 spec.toon 迁移 fixture 中 `run llman sdd status`
  字样为历史数据，仅在不耦合行为时保留原样（评估后决定）。

## Capabilities

- `cli`：删除 r42 status 命令合约。
- `sdd-workflow`：r1 / r93 / r126 查询面措辞收敛（show/list）。

## Impact

- 受影响范围：CLI 命令面（-1 子命令）、双语 skill 模板 3 文件 × 2 locale、
  渲染产物 resync、i18n key 清理、2 个测试文件、2 处 AGENTS.md 文档。
- 兼容性：`llman sdd status` 直接消失（breaking）；替代路径
  `show <id> --json --type change` 与 `list --json` 均已存在且字段超集。
  `list --json` 自身的 `status` 字段（change 生命周期态）与本命令无关，不动。
- 不涉及：config schema、index/context、模板渲染协议、其他子命令语义。

## Open Questions

无——scope 已由探索阶段确认（不做 `change diff --summary`，status 直接移除）。