---
depends_on: []
branch: sdd/rework-finalize-merge-semantics
base_sha: 5d38307d575673b2fcc56cc303b67cdcfe90d854
---

# finalize 合并语义 v2：squash 默认 + 目标可解析 + worktree 拓扑守卫

## Why

`change finalize`/`archive` 的收口合并存在三重问题（关联 issue #19）：

1. **合并目标硬编码默认分支**：`do_ff_merge` 用 `resolve_default_branch_ref` 解析目标，无 `--into`、无配置。live spec（sdd-workflow r113）早已写明目标是「分叉点分支（一般是默认分支）」，实现与 spec 措辞存在偏差；stacked 分支工作流下语义不成立。
2. **worktree 并行下自动合并必然失败且静默降级**：linked worktree 中 `git checkout <default>` 被 git 拒绝（目标分支被主 worktree 持有）；即使成功，main 已前进时 `--ff-only` 也必然失败。失败后仅 eprintln 一句提示，照常 rename + auto-commit 并报告 archived——r113 的 best-effort 在该拓扑下不是例外而是常态，用户误以为 specs 已落库。
3. **散列 commit 噪音**：ff-merge 把 feature 分支上的全部 WIP commit 原样带进基准分支，一个 change 在 main 上留下 N+1 个低信息量 commit。

llman 自身把 `change start --worktree`（r116）作为一等特性宣传，但该流程下 finalize 无法闭环——工具内部不一致，必须修复。

## What Changes

- **绑定补全**：`change start`/`attach` 写 binding 时追加 `base_branch`（fork 基准分支）。start 语义恒为默认分支；attach 缺省默认分支、可 `--base <branch>` 显式覆盖（stacked 场景）。旧绑定缺该键 → 回退默认分支（零迁移）。
- **合并目标解析**：finalize/archive 新增 `--into <branch>`；目标解析优先级 `--into` > binding `base_branch` > 默认分支（兜底）。
- **合并方式默认 squash（破坏性默认值变更）**：新增 config `sdd.merge_method`（`squash`（缺省）| `ff`）与 CLI `--method` 覆盖。squash 路径下「squash 暂存实现 diff + docs rename + 自动提交」塌缩为基准分支上**单个 commit**（沿用 `archive(sdd): <id>` 收尾 message）。`ff` 保留旧语义（原样带入 feature commits + 1 个收尾 commit）。ff-only 对已分叉目标必然失败的缺陷在 squash 下不复存在。
- **worktree 拓扑守卫**：checkout 前用 `git worktree list` 检测目标分支是否被其他 worktree 持有；持有时跳过自动合并，输出可执行的手动命令指引，降级显式化（警告而非静默）；docs rename 仍照常完成（保留 r113「不因合并失败回滚 rename」）。
- **归档后停留目标分支**：成功时留在合并目标（一般为默认分支），r94「留在默认分支」措辞推广为「留在目标分支」。

### 破坏性说明与升级路径

- 破坏面仅限**默认值翻转**（ff → squash）：不删除/重命名任何字段、命令、tag 或 stage 值域，旧行为经 `sdd.merge_method: ff` 完整可达 → 按 r28 不设 `migrations/v<from>-v<to>/`，README + 本节承担告知义务。
- 与 draft change `archive-commit-message-configurable-per-project-or-global` 正交（该提案管 message 模板，本 change 管合并语义）；本 change 先落，其正式化时在其上改 r94 的 message 描述。

## Capabilities / Impact

- `sdd-workflow`：r111（binding 写入含 base_branch）、r113（合并语义 v2 主条文）、r124（frontmatter 合法字段集 + base_branch）
- `sdd-bdd-mode-compat`：r57（binding 结构）、r7（archive 统一行为）、r94（finalize 流程）
- `sdd-structured-skill-prompts`：r98（apply-cycle 技能「本地合回基准分支」步骤措辞）
- 代码面：`crates/llman-sdd/src/sdd/change/{archive,finalize,git_native,start}.rs`、`config.rs`（FlowConfig + schema 源）、locales 双份、`crates/llman-sdd/templates/sdd/**` 技能模板、根 AGENTS.md、README（`just readme`）
- 明确非目标：`change diff`/lock-gate 的范围计算不变（仍为与本地默认分支的 merge-base，r137）；`base_branch` 仅作合并目标解析，不参与范围语义；不自动在持有 worktree 内执行合并（风险评估：他方 agent 可能正在操作中，手动命令等效且安全）。

## 测试边界（seam）

复用既有 seam，不发明新接缝：

1. **BDD harness seam**：`tests/bdd_steps.rs` 泛化 step 驱动 `llman` CLI 子进程（`@executable` 经 `bdd.bindings tags:[executable]` 绑定）；新增/调整场景只复用既有 step 词汇。
2. **Rust 测试 seam**：`archive.rs`/`finalize.rs`/`git_native.rs`/`start.rs` 内嵌单元测试（既有 tempfile git 仓 helper）；`tests/it/sdd_bdd_compat.rs` 兼容断言（finalize 语义、命令面 smoke）。
