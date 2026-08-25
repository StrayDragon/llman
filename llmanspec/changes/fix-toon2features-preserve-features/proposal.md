---
depends_on: []
rules_edit_acked: true
branch: sdd/fix-toon2features-preserve-features
base_sha: ddf37cd14adbe04c5fe73f09e0565006eb584a38
checkpointed: false
---

## Why

worktree 回放（`wt/toon2features-replay`，基点 a275d6a）暴露 toon2features 语义错误：当前实现把 capability 目录中**既有的多文件 `.feature` 合并进主文件后删除**。这些文件是绑 harness 的活资产（`scenarios!("llmanspec/specs", tags="@executable")` 直接驱动），机器合并/删除正是 v0.0.67 迁移丢失 39 个可执行场景、需要两轮 fix 召回的根因。正确职责边界：迁移只消费遗留 `spec.toon`，既有 `.feature` 一律不动，由人工按 r131 合并。

## What Changes

- `llman sdd project migrate --kind toon2features` 语义重写（spec-format r136 改写，锁定规则编辑已 ack）：
  1. 目录中既有 `*.feature` 文件 MUST NOT 被读取改写或删除；迁移报告 MUST 计数 `left` 并提示按 r131 人工合并。
  2. 仅处理 `spec.toon`：`requirements[]` → `@req:<id> @human`（statement 原文入描述，不变）；`scenarios[]` 中含 GWT 内容（given/when/then 任一非空）且 req_id 配对的行 → `@req:<id> @human` 场景（id 入标题，步骤关键字按检测语言渲染，遗留关键字前缀剥离）；req_id 未配对 → `dropped_unpaired`；无 GWT 内容 → `dropped_notes`；`feature` 列仅作历史记录不再分支。
  3. 移除 feature=true → `@executable` 转写路径（toon 行是文档性注记，可执行验收活在既有 `.feature`）。
  4. 已存在同名 `<capability>.feature` 时 MUST 跳过该 capability（保留 spec.toon，警告人工合并后重跑）。
  5. Gherkin 语言检测链：config `bdd.default_language` > config `locale` 映射 > 任一既有 `.feature` 的 `# language:` 头 > 英文兜底。
- BDD 场景同步改写（spec-format executable）：converts-and-cleans 改用仅 toon fixture；embedded 场景改为「不动 .feature + GWT 行转 @human + 语言渲染」；新增 skip 场景。

## Capabilities

- `spec-format`（r136 迁移合约）
- `sdd-project` 行为面（migrate 子命令，无独立 spec，由 spec-format 承载）

## Impact

- 代码：`src/sdd/project/migrate.rs` 重写转换核心；`src/sdd/spec/backend/feature_backend.rs` 增语言感知 dump；`tests/bdd_steps.rs` fixture 调整。interop（`dump_main_spec` 默认 zh-CN）不变。
- 迁移后单轨 `validate` 对遗留多 `.feature` 目录会如实报 r131 合并错误——诚实中间态，报告明示。
