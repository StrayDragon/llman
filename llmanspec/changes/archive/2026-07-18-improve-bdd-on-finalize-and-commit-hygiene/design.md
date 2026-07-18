# Design: BDD-on 单 commit 收尾（`finalize`）

> 对应 proposal.md「Route C：单 commit + 弱化 sha」。本文记录关键决策、备选方案
> 权衡与失败语义实现思路，供 apply 阶段直接落地。

## D1：为何接受 `checkpoint_sha` 弱化（显式 supersede 旧 Non-goal）

**问题**：单 commit 与「`checkpoint_sha` 精确指向实现 commit」不可兼得。

- `checkpoint_sha` 的旧语义是「实现 commit 的 HEAD」。读 HEAD sha 用 `git rev-parse HEAD`，
  在脏树下也能拿到，但拿到的是**上一个** commit 的 sha，不是即将发生的「实现 + 收尾」commit。
- 方案 A（amend 回填）能保留严格 sha，但要 finalize 接管 `git commit` + `git commit --amend`，
  复杂度爆炸（见 D2）。
- 方案 C（本提案）放弃严格 sha，让 `checkpoint_sha = base_sha`，审计链仍可由
  `git diff base_sha..HEAD` 重建。

**取舍依据**：

1. `base_sha` 在 attach 时已写入（`git_native.rs:266-310` 的 `run_attach`），已是审计链的
   一部分；finalize 复用它不引入新概念。
2. `checkpointed: true` + `checkpoint_sha = base_sha` 的组合在 archive gate 中只需检查
   `checkpointed == true` 即可（已是这样，`git_native.rs:446`），无需新增校验逻辑。
3. 失去「实现是否被改动过」颗粒度，可由 archive 后的 `git log/diff` 弥补；BDD-on 的 live specs
   本身在 feature 分支上由 Git merge 提升，commit 历史本就是审计源。

**结论**：supersede 旧 Non-goal「不削弱 `checkpoint_sha` 审计」。新增文档与 help 文本必须明确
说明两种模式（`checkpoint` 写实现 HEAD / `finalize` 写 base_sha）的差异。

## D2：备选方案为何被否决

| 方案 | 闭环 commit | sha 语义 | 接管 git? | 否决理由 |
|---|---|---|---|---|
| A：单 commit + amend 回填 | 1 | 实现的 HEAD（amend 后） | 是（commit + `--amend`） | finalize 接管 git：需传 `-m` 或读 EDITOR；amend 改写历史（feature 分支若已 push 需 force-push）；commit 成功但 amend 失败的中间态难以回滚。 |
| B：放宽 archive gate（2 commit） | 2 | 实现的 HEAD（不动） | 否 | 用户已明确要单 commit。 |
| 原 draft Option 2：放宽 archive gate | 2 | 实现的 HEAD（不动） | 否 | 同上。 |
| 原 draft `finalize --also` 批量 | 1×N | base_sha | 否 | scope creep；单条稳定前不做。 |

## D3：为何不检查 clean tree（finalize 的核心）

`run_checkpoint` 在 `git_native.rs:341-343` 要求 clean tree，原意是：让 `checkpoint_sha`
指向一个**确定已 commit** 的状态，避免脏树里的实现被后续覆盖而 sha 失真。

finalize 不需要这个保证，因为它写的 sha 是 `base_sha`（attach 时已固定），与当前工作区
状态无关。去掉 clean-tree 检查正是单 commit 的关键 —— 实现可以留在脏树里和 finalize 的
frontmatter + rename 一起一次性 commit。

**风险**：用户可能在实现不完整（半成品代码、未保存 buffer）时误调 finalize。缓解：

- finalize 仍跑 `validate`（除非 `--no-check`），live specs strict 门禁会拦截半成品 spec；
- 但 validate 不检查产品代码完整性（只检查 specs/features/工件一致性）—— 这是**已知缺口**，
  通过文档明确「finalize 调用前 agent 应自行确认实现已自测」。

## D4：失败语义与幂等性

按发生顺序（每步失败 = 前面已写内容的状态）：

| 失败点 | 已写入内容 | 工作区状态 | 重试行为 |
|---|---|---|---|
| Gate 检查（attach/branch/default/feature_delta） | 无 | 原样 | 直接重试 |
| `validate` 失败（live 或 change stage） | 无 | 原样 | 直接重试 |
| 写 frontmatter（`write_binding`）失败 | 无（atomic_write） | 原样 | 直接重试 |
| archive rename 失败（目标已存在/IO） | frontmatter 已写 | 仅 frontmatter 脏 | **幂等重试**：检测 `checkpointed: true && checkpoint_sha.is_some()` → 跳过写 binding → 直接尝试 rename |

**幂等检查实现思路**（apply 阶段细化）：

```text
读 binding
  if binding.checkpointed && binding.checkpoint_sha.is_some():
      println!("change already checkpointed; proceeding to archive rename")
      skip to archive step
  else:
      run gates + validate + write_binding
  end
archive rename (BDD-on docs-only)
```

注意：旧 `enforce_bdd_archive_gates` 仍检查 `working_tree_clean`，finalize **不能**直接调用它
（否则又走回老路）。需要抽取一个不含 clean-tree 检查的 gate 变体，或 finalize 直接调用
archive 的 rename 段而不走 `enforce_bdd_archive_gates`。

## D5：finalize 与 archive 的代码复用

现有 `archive::run_with_root`（`archive.rs:48-167`）做了：

1. （BDD-on）调 `enforce_bdd_archive_gates` + 警告 leftover TOON deltas；
2. （BDD-off）合并 TOON delta；
3. `fs::rename(changes/<id>, archive/YYYY-MM-DD-<id>)`。

finalize 只需要 BDD-on 路径，且需要**绕过** step 1 的 clean-tree 检查。两条路线：

- **路线 1（抽取共享函数）**：把 `enforce_bdd_archive_gates` 拆成
  `enforce_bdd_archive_gates_strict`（含 clean tree，旧 archive 用）和
  `enforce_bdd_archive_gates_relaxed`（不含 clean tree，finalize 用）。archive rename 段
  抽成 `do_bdd_on_archive_rename(root, change_id)` 供两边复用。
- **路线 2（finalize 直接 inline）**：finalize 自己跑 gate + 自己调 `fs::rename`，不共享代码。

**推荐路线 1**：避免 rename 路径名约定（`YYYY-MM-DD-<id>`）漂移；relaxed gate 后续可能用于
其他场景（如 r57 的 archive gate 也可能放宽）。apply 阶段细化具体函数签名。

## D6：为何不做 `--dry-run`

`checkpoint --dry-run` 和 `archive --dry-run` 都有意义（前者预览将写的 frontmatter，后者
预览 rename）。但 finalize 的语义是「同进程 checkpoint + archive」，dry-run 必须同时**不写
frontmatter 且不 rename**，结果只是打印一句「会做这些」—— 与直接读 proposal.md 的
`checkpointed` 字段和 archive 目标路径名（`YYYY-MM-DD-<id>`，可预测）相比没有信息增益。

如果未来证明有需求（例如 CI 想预检 finalize 会成功），再加，初始版本不做。

## D7：合约归属（BDD-on Partitioned SSOT）

按 AGENTS.md「BDD 模式兼容性测试维护规则」：

- **约束层**：`llmanspec/specs/sdd-bdd-mode-compat/spec.toon` 新增 `r94` requirement
  （finalize 行为合约），并加一行不可执行 scenario 指向 .feature。
- **Harness 层**：`llmanspec/specs/sdd-bdd-mode-compat/git-binding.feature` 新增 `@req:r94`
  可执行场景（泛化 step 库可覆盖：运行 llman → 断言退出码 / stderr）。
- **实现细节层**：`tests/sdd_bdd_compat_tests.rs` 的 13 子命令 smoke 列表新增 `Finalize`
  （`read_only: false`）。
- **Skills 同步**：`llman-sdd-archive` skill 加 finalize 推荐路径；`llman-sdd-apply` /
  `verify` 加「勿单独 commit 纯 draft」注记。

r94 全局唯一性已由 `llman sdd spec next-req-id` 分配确认。

## D8：开放问题（apply 阶段决定，不阻塞 propose）

1. **finalize 是否要求 binding 的 `checkpointed` 为 false？** —— 倾向**不要求**（幂等重试
   场景就是 `checkpointed: true` 进来）。但首次正常流程也允许 `checkpointed: true`（用户
   先单独 checkpoint 过）—— 此时 finalize 应跳过 binding 写入，直接 archive。
2. **finalize 在 BDD-off 下的行为？** —— 倾向**拒绝**（与 `checkpoint` 一致，提示需 BDD-on）。
   BDD-off 没有 attach/checkpoint 流程，finalize 无意义。
3. **`--no-interactive` 是否真的完全忽略？** —— 是，对齐 `checkpoint --no-interactive`
   的处理（`command.rs:477-479` 的注释已说明）。
