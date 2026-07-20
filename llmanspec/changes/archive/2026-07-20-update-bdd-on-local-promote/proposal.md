---
depends_on: []
branch: bdd-on-local-promote
base_sha: c63e02a9876d8f5956ff3d062d9313acc9d9d671
checkpointed: true
checkpoint_sha: c63e02a9876d8f5956ff3d062d9313acc9d9d671
---

## Why

BDD-on（Git-native Partitioned SSOT）的真实收尾闭环是本地 `git merge --ff-only`，Hosting PR / `git push` 仅在需要远程审查时才用。但当前多处 skill 模板与 docs 把收尾叙事写成「Git/PR merge」「打开/合并 feature 分支 PR」，导致 agent 默认执行 `git push -u` + `gh pr create/merge`，对个人仓/本地 ff-merge 的下游（如 xylitol，2026-07-20 反馈）造成误导并浪费 token。

更深层的问题：现有合约 `sdd-structured-skill-prompts` r65 已用中性「Git merge」措辞，但实现（模板正文）偏离为「Git/PR merge / 打开 PR」——本变更是**让实现回归合约**，并补 P1 增量（finalize 成功提示、validate BDD-on next-steps 去误导）。

明确不做（本轮）：不引入 `llmanspec/config.yaml` 的 `sdd.promote.style: local|pr` 配置——那需要元 skill / 动态 skill 按项目条件渲染不同收尾段；现状模板是静态的，加配置也无法让已安装 skill 条件分支。留待后续元 skill 机制。

来源：`SUGGESTION-bdd-on-promote-local.md`（xylitol 下游摩擦）。

## What Changes

### P0 — 模板/技能叙事统一（中性化「Git merge」，去 PR/push 默认导向）

- `templates/sdd/{zh-Hans,en}/skills/llman-sdd-archive.md`
  - `description`：`Git/PR merge promotes live specs` → `merge promotes live specs`（去「PR/」）
  - 正文「再经 Git/PR merge 提升」「Git/PR merge」「打开/合并 feature 分支 PR」→ 统一为「本地 merge 进默认分支（push/PR 可选）」
- `templates/sdd/{zh-Hans,en}/skills/llman-sdd-sync.md`
  - 「Git/PR merge」→ 中性「merge 进默认分支」
- `templates/sdd/{zh-Hans,en}/units/spec/toon-contract.md`
  - 「Git/PR merge promotes specs」→ 中性「merge promotes specs」
- `templates/sdd/{zh-Hans,en}/skills/llman-sdd-apply-cycle.md`
  - 「4) 提交」后新增「5) 本地合回默认分支」步骤（`git switch <default> && git merge --ff-only <feature>`，可选 `git branch -d <feature>`）
  - 硬约束新增：未获用户明确要求时，禁止 `git push` / `gh pr create|merge`
- `AGENTS.md`：「正常 Git/PR 合并」→「正常 Git 合并（本地 merge 即可）」
- `docs/sdd/pipeline-bdd-on.md`：流程图终点 `Git/PR merge feature → 默认分支` 与 `PR merge` → 统一为本地 merge 语义
- `.agents/skills/**`：跑 `llman sdd init --update` 重新生成渲染副本

### P1 — CLI 用户可见输出

- `src/sdd/change/finalize.rs`：成功 `println!` 后追加一行 next-step 提示（走 `t!` 本地化键，与 archive 一致），例如：
  `Next: commit on the feature branch, then merge into <default> locally (push / hosting PR optional).`
- `src/sdd/shared/validate.rs::print_next_steps`：BDD-on 下对 `ItemType::Change` 失败时，MUST NOT 无条件打印 `change_step_1`（「Ensure change has deltas…」——BDD-off 残留）。改为按 BDD 模式分支：BDD-on 指向 live specs + attach/finalize；BDD-off 保留 delta 提示。相应 locale key 在 `locales/app.yml` 调整或新增。

### P2 — 轻量 draft 提案路径 + change id 自动推导（合并新增）

来源：用户反馈（2026-07-20）。当前 propose skill 硬约束「必须与用户确认 change id 后再写文件」对「快速记一个提案」场景过重；`llman sdd change new <CHANGE>` 要求必填 id，无推导能力。

- **skill 层**：`templates/sdd/{zh-Hans,en}/skills/llman-sdd-propose.md` 新增「轻量 draft 路径」节。当用户意图为快速起草提案且未提供 id 时：MUST NOT 询问确认 id；MUST 从描述内容直接生成合法且有意义的 id（遵循 `llmanspec/AGENTS.md` 命名约定，若无则按语义合理命名）；仅 `llman sdd change new` 建 proposal.md。完整 propose（triage + tasks + specs + attach）仅在用户明确要求正式化时启动。
- **CLI 层**：`src/sdd/change/new.rs` 增强——支持 `--from <description>`，由 CLI 生成 id 并在 stdout 打印最终 id + proposal 路径；冲突既有 change 时非零退出并提示 `--force`/换描述。`<CHANGE>` 位置参数在提供 `--from` 时可选。

## Capabilities

- **`sdd-structured-skill-prompts`**（r65 扩展 + r98 新增 + r99 新增）：skill 模板收尾叙事不导向 PR/push；轻量 draft 路径 + change id 自动推导。
- **`sdd-bdd-mode-compat`**（r98 关联）：finalize 成功提示与 validate next-steps 在 BDD-on 下的行为约束。
- **`cli-experience`**（r43 既有）：finalize 成功消息改走 `t!` 本地化键（既有合约的合规修复，不新增 requirement）。

## Impact

- **用户可见**：BDD-on 项目下 agent 不再默认 push/PR；finalize 成功后看到本地 merge 提示；validate 失败时 BDD-on 不再误导去写 delta；说「draft 提案」时 agent 直接生成 id 并建 proposal.md，不再反问 change 名。
- **下游**：xylitol 一类个人仓/本地 ff-merge 用户的 token 消耗与摩擦降低。
- **向后兼容**：不改任何命令的退出码语义或 flag 矩阵（`change new` 新增可选 flag，既有调用不变）；文案与提示调整；BDD-off 路径完全不变。
- **测试**：新增 `.feature` 场景驱动 P0（模板措辞）+ P1（finalize 提示 / validate next-steps）+ P2（change new --from 推导）；`tests/sdd_bdd_compat_tests.rs` smoke 列表无需改动（未增减子命令）。
