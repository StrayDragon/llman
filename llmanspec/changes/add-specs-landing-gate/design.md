# Design：Specs landing

## Concepts（勿与 start 混淆）

| 概念 | 职责 | CLI 信号 |
|------|------|----------|
| Change 壳 | `changes/<id>/{proposal,design,tasks}` | Draft → Designed |
| Branch binding | `change start` / `attach` → `branch` + `base_sha` | Full（stage） |
| Specs landing | 绑定分支 tip 相对 `base_sha`，`llmanspec/specs/**` 有 path 变更（已进该分支历史） | `specsLanded` |
| Apply-ready | Full ∧ (landed ∨ skip) | `readyToImplement` |

`change start` **只**做 binding（干净树、从默认分支建 `sdd/<id>`）。不创建 specs，也不表示可 apply。

丢失「分支上的 specs 改动」→ **恢复**（checkout 绑定分支重写/重建分支 + `attach --force`），**不要**再跑 `start`（已 attach 会拒绝）。

## Specs landed 判定

```text
git diff --name-only <base_sha>...<binding.branch> -- llmanspec/specs
```

- 非空 → `specsLanded: true`
- 绑定分支不存在 / git 失败 → `false`，stderr/字段旁路用简短原因
- 看的是 **binding.branch tip**，不是当前 HEAD（在 main 上 `show` 仍能诊断）
- 仅工作区脏、未 commit → **不算** landed（硬门禁要求 commit）；validate 可另 WARNING「脏 specs 在默认分支」

## 豁免

Frontmatter `skip_specs_landing: true`：治理/纯 `changes/` 文档、明确不改 live 合约时。合法字段集（r124）纳入该键。默认缺省 = 需要 landing。

## readyToImplement（修订 r93）

```text
readyToImplement = (stage == Full) && (specsLanded || skip_specs_landing)
```

`determine_stage` 三态不变；只改 ready 布尔。

## 友好错误（token 短 + skill 路由）

未落地示例：

```text
specs not landed: change `ID` bound to `BRANCH` but base...BRANCH has no llmanspec/specs/ changes.
Edit live specs on that branch and commit. Skill: llman-sdd-propose (land specs) — do NOT re-run change start if already attached. Apply only when show --json readyToImplement=true (llman-sdd-apply).
```

默认分支脏改 specs：

```text
live specs dirty on default branch: commit/stash is wrong place — switch to bound sdd/<id> (or change start) before editing llmanspec/specs/. See AGENTS.md Specs landing. Skill: llman-sdd-propose.
```

## Soft vs hard

| 层 | 行为 |
|----|------|
| Soft | `validate`：Full ∧ !landed ∧ !skip → **WARNING**（含 skill 引导） |
| Soft | `show`/`status` JSON/文本含 `specsLanded`、`skipSpecsLanding` |
| Hard | `readyToImplement` 按上式；apply skill 读 show，false 则 STOP |

本 slice **不**新增独立 `llman sdd apply` 子命令门禁（apply 是 skill）；硬门禁落在 show 布尔 + skill 契约。若后续有 CLI apply hook 再复用同一函数。

## Propose 顺序（文档）

```text
change new → proposal/design/tasks
  → change start | attach
  → edit llmanspec/specs/** on bound branch → commit
  → apply …
```

澄清 r111 措辞：实现是 **必须从默认分支执行 start**（已在非默认则 attach）；「拒绝绑到默认分支」属 attach。本 change 修正 skill/AGENTS/约束层说明与实现一致。

## 测试 seam

- `llman sdd show <id> --type change --json`（specsLanded / readyToImplement）
- `llman sdd status`（或同构 JSON）
- `llman sdd validate <id>`（WARNING）
- 泛化 BDD step + 必要时扩展 fixture（specs commit 或 skip 旗标）
