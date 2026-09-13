# Design: finalize 合并语义 v2

## D1 合并目标解析：`--into` > `base_branch` > 默认分支

- `ChangeGitBinding` 增加 `base_branch: String`（proposal.md frontmatter 第三个 CLI 写入键）。`read_binding` 对缺键的旧 proposal 返回 `base_branch = ""`，调用侧空值回退默认分支——**不做 schema 迁移**。
- `change start`（含 `--worktree`）：要求在默认分支上（r111 不变），`base_branch` 恒写默认分支名。
- `change attach`：缺省写默认分支；新增 `--base <branch>` 显式记录 stacked fork 源。不做 reflog 推测（脆弱、跨机器不可复现）。
- `attach --force` 重绑时按当前状态重算（与 branch/base_sha 同步刷新）。
- 解析结果允许指向非默认分支（stacked：finalize 把 feat2 并回 feat1），target 不做默认分支偏好校验。

## D2 合并方式：squash 缺省，ff 可选

- config：`sdd.merge_method: squash|ff`（FlowConfig 新字段，serde default → `squash`）；CLI `finalize`/`archive` 新增 `--method <squash|ff>`，flag > config > 内置缺省。
- squash 路径：`git checkout <target>` → `git merge --squash <feature>` → docs rename → 单次 `git commit`（message 沿用现有收尾文案 `archive(sdd): <id>`）。基准分支上 **一个 change = 一个 commit**，WIP 散列 commit 不再外泄。
- ff 路径：现行为原样保留（feature commits + 1 个收尾 commit），`--ff-only` 语义不变。
- 破坏性：默认值翻转。r28 迁移目录不设（无移除/重命名，旧行为经配置完整可达）；README 命令表与 AGENTS.md 生命周期表同步措辞。

## D3 worktree 拓扑守卫：显式降级，不代持分支者合并

- checkout 前执行 `git worktree list --porcelain`：若目标分支已被**其他** worktree 持有 → 跳过自动合并，stdout 输出可执行指引（如 `run manually in <path>: git switch <target> && git merge --squash <feature> && git commit` 或对应 ff 形式），并以显著 WARNING 说明「归档文档仅落在当前分支，基准分支未收口」。
- 降级后 docs rename 与收尾照常（保留 r113「不因合并失败回滚 rename」与幂等判定：目录在 changes/archive/ 即已归档，不引入 frontmatter 存档字段）。
- **否决**「在持有 worktree 内自动执行 merge」：他方 agent 可能正在该树操作，代为变更状态风险大于收益；打印命令等效且零风险。**否决**归档 frontmatter `merged: false` 标记：幂等判定已明确禁依赖存档字段（r94），信息用输出承载即可。

## D4 边界情况

- **脏树**：现有 `stash_if_dirty`/`pop_stash_if` 机制沿用到新目标分支；squash 暂存（feature tip diff）与 stash pop（未提交 diff）在 commit 时合一，重叠文件以 pop 后工作树为准。
- **零提交 change**：feature tip == merge-base 时 `merge --squash` 报 "Already up to date"，暂存为空；rename + 收尾 commit 照常落目标分支（等价于纯 docs 提交）。
- **目标已包含 feature（重复 finalize 场景）**：squash 暂存为空，rename 照常；不视为错误。
- **checkout 目标失败但非 worktree 占用**（如本地不存在该分支引用）：与现行为一致——eprintln 指引 + 跳过合并 + 继续 rename，但统一走 D3 的显式警告通道。

## D5 测试策略

- 单元：`git_native.rs`（base_branch 写入/读取/缺键回退、attach --base、--force 重算）、`archive.rs`（squash 单 commit 形态、目标解析优先级、worktree 占用降级、降级后 rename 不回滚）、`finalize.rs`（r94 流程改写后的门禁与收尾、留在目标分支）。
- 兼容：`tests/it/sdd_bdd_compat.rs` smoke 面（finalize 语义相关断言适配）。
- BDD：`sdd-bdd-mode-compat.feature` 既有 @executable 场景保持可驱动；新行为以 Rust 测试为主承载，新增 @executable 场景仅当能复用既有泛化 step 词汇。
