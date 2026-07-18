---
id: improve-bdd-on-finalize-and-commit-hygiene
stage: draft
depends_on:
- fix-sdd-bdd-on-change-stage
- improve-partitioned-ssot-agent-friction
branch: propose/improve-bdd-on-finalize-and-commit-hygiene
base_sha: 97eca13f0f6a2c183d4d978b7a2bb56f6f4767a7
checkpointed: true
checkpoint_sha: 97eca13f0f6a2c183d4d978b7a2bb56f6f4767a7
---

# Proposal: BDD-on 单 commit 收尾（`finalize`）

## Why

Consumer（xylitol）在 `feat/tui-dev` 上一日多闭环后统计：未 push ~50 commit 中约一半是
`chore(sdd)/docs(sdd)` 的 **draft / checkpoint / archive**。产品 `feat`/`fix` 细粒度合理；
摩擦来自 BDD-on 硬时序——每个闭环**结构性 +2** 流程 commit：

```text
commit(code+specs) → checkpoint → commit(metadata) → archive → commit(rename)
```

根因（已在代码层确认）：

- `run_checkpoint`（`src/sdd/change/git_native.rs:341-343`）要求 clean tree，写完
  `checkpointed`/`checkpoint_sha` 后又使工作区变脏；
- `enforce_bdd_archive_gates`（`git_native.rs:443-445`）再次要求 clean tree → 中间必须
  commit 一次纯 metadata；
- archive 自己产生 rename 脏树 → 又得 commit 一次。

`0.0.64` 的 `improve-partitioned-ssot-agent-friction` 把这条时序写进了 skill，但
**没有**提供「收尾一次 dirty → 一次 commit」的 CLI，agent 只能机械拆提。

Field notes：`docs/release/partitioned-ssot/COMMIT_FRICTION_FROM_XYLITOL.md`。

## What Changes

> Route C：单 commit + 弱化 sha（详见 design.md D1/D2）。

### 1. 新子命令：`llman sdd change finalize <id> [--no-check] [--no-interactive]`

**MUST**：同进程执行 checkpoint 门禁 → 写 `checkpointed` / `checkpoint_sha` → docs-only archive
（`rename changes/<id> → changes/archive/YYYY-MM-DD-<id>/`），结束后工作区脏内容 =
「实现 diff + frontmatter 改写 + archive rename」，**一次** `git commit` 即可收尾。

**Gate 矩阵**（BDD-on，区别于旧路径）：

| 门禁 | 旧 `checkpoint` | 旧 `archive` | 新 `finalize` |
|---|---|---|---|
| 已 attach（branch + base_sha 非空） | MUST | MUST | MUST |
| 当前分支 == `binding.branch` | MUST | MUST | MUST |
| 非默认分支 | MUST | MUST | MUST |
| 工作区 clean | **MUST** | **MUST** | **MUST NOT 检查** |
| 跑 `validate` 门禁（live strict + change stage） | MUST | — | MUST（除非 `--no-check`） |
| `checkpointed == true`（archive 前置） | — | MUST | 由本命令写入 |
| 无遗留 `*.feature.delta.toon` | — | MUST | MUST |

**MUST NOT**：finalize 不接管 `git commit`；不调用 `--amend`；不写除 frontmatter + archive
rename 之外的任何文件；不与 `--dry-run` 共存（finalize 语义要求原子写入，dry-run 无意义）。

**Flag 语义**：

- `--no-check`：跳过 `validate` runner（与 `checkpoint --no-check` 同义）。
- `--no-interactive`：**接受并忽略**（对齐 change 子命令 flag 矩阵；finalize 本身无交互逻辑）。

### 2. `checkpoint_sha` 语义弱化

finalize 模式下：`checkpoint_sha = binding.base_sha`（即 attach 时记录的 merge-base）。

- 旧路径 `checkpoint` 写入的 sha 仍 = checkpoint 调用前的 HEAD（实现 commit）。
- finalize 写入的 sha 不再精确指向实现 commit，因为单 commit 模式下实现 commit **尚未发生**
  （finalize 调用后才会 `git commit`）。
- 审计链保留：`base_sha` + `branch` + `checkpointed: true` 仍可重建实现范围
  （`git diff base_sha..HEAD`）。

### 3. 失败语义（MUST）

- Gate 失败（未 attach / 默认分支 / 遗留 feature_delta）：MUST 在任何写入前退出非零，
  **不**改 frontmatter、**不**移动文件。
- `validate` 失败：MUST 在 archive 前退出非零；frontmatter **尚未写入**（因为先 validate
  后写 binding）。
- archive rename 失败（目标已存在 / IO 错误）：MUST 退出非零；此时 frontmatter 已写但未 commit，
  工作区脏内容仅含 frontmatter，agent 可直接 `git checkout -- proposal.md` 回滚或重试 finalize。
  **finalize MUST 是幂等的**：重试时发现 frontmatter 已含 `checkpointed: true` 且 `checkpoint_sha`
  非空时，跳过写入并直接尝试 archive rename。

### 4. 旧路径保留

`llman sdd change checkpoint` + `llman sdd change archive`（含双重 clean-tree 门禁）**完全
不变**，作为显式多 commit 工作流的 fallback。两条路径可混用（例如先 checkpoint 再 finalize
会被 finalize 的幂等检查识别）。

## Skills / 文档

- `llman-sdd-archive`：新增推荐路径 `finalize`（单 commit 收尾），保留旧 5 步时序作 fallback。
- `llman-sdd-apply` / `llman-sdd-verify`：明确**不要单独 commit 纯 draft**；draft 与 propose
  / 首实现同提。
- `docs/release/partitioned-ssot/UPGRADE_AGENT_PROMPT.md`：在落地后补充 finalize 章节。
- 本提案 field notes 链到 release 文档。

## Non-goals（含对原 draft 的调整）

- 不强制 squash 产品实现 commit（仅减少流程 chore）。
- 不取消 `checkpoint_sha` 审计字段 —— **但明确接受**：finalize 模式下其颗粒度从「实现 commit sha」
  降为「base_sha」。**原 draft 的「不削弱 `checkpoint_sha` 审计」Non-goal 在本提案中显式 superseded**，
  理由：单 commit 与严格 sha 二选一不可兼得（详见 design.md D1）。
- 不改 Partitioned SSOT（live specs 仍在 feature 分支上由 Git/PR 提升）。
- 不做 `finalize --also <id2>,<id3>` 批量收尾（scope creep；等单条稳定后另议）。
- 不做 `finalize --dry-run`（语义要求原子写入，dry-run 无意义）。
- 不改 `checkpoint` / `archive` 旧路径的任何行为（仅新增并行路径）。

## Impact

- Agent/人类每 BDD-on 闭环从 3 commit 降到 **1 commit**。
- Consumer（xylitol）可删 `llmanspec/AGENTS.md`「提交卫生」里对 finalize 缺口的说明。
- 调用方需知晓：finalize 写入的 `checkpoint_sha` 是 base_sha，不是实现 commit sha。

## Acceptance sketch

- BDD-on fixture：实现完成且工作区脏 → `llman sdd change finalize <id>` 成功退出 →
  `llman sdd list` 无该 active change → `changes/archive/YYYY-MM-DD-<id>/` 存在且含
  `proposal.md`（`checkpointed: true`，`checkpoint_sha` == `base_sha` 非空）→ 一次
  `git commit` 后工作区干净。
- Gate 失败：未 attach → 退出非零、frontmatter 未改、目录未移动。
- 幂等：archive rename 失败后重试 `finalize <id>` → 检测到已 checkpointed → 直接尝试 rename。
- 兼容：旧 `checkpoint` + `archive` 路径（双重 clean-tree 门禁）行为不变。