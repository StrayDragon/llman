# Design：Git-native 语义 v2

自包含设计：实现者只需本文件 + proposal + tasks。决策来自 2026-09-11 探索会话（已由需求方拍板 D1✅ D2✅ D3✗ D4✅quick 已完成 D5→二期 draft）。

## 1. 决策记录

| # | 决策 | 结论 |
|---|------|------|
| D1 | 范围语义锚点 | 四处消费（锁定门禁/landing/diff/staleness）全部改**现算** merge-base(本地默认分支, HEAD)；存储 base_sha 退役为审计字段 |
| D2 | ack 粒度 | `rules_edit_acked: bool` → `rules_touched: [req-id]`；bool 读取兼容 |
| D3 | 状态存储 | **否决** sidecar；状态留 proposal frontmatter（change 自表达） |
| D4 | EOF 换行 | quick 已完成（`1a18f7c`，split_frontmatter 规范化尾换行 + 幂等测试） |
| D5 | 生命周期减法 | 二期，draft `lifecycle-self-expressive` 记录方向，本 change 不做 |
| G1 | gate 落点 | **不新增命令**；挂在 `show <change>`（JSON gateChecks + text 紧凑 Gates 段） |
| G2 | token-efficiency | show 默认输出只给「计数行 + 未过项」；全量细节仅按需 |

## 2. D1 换锚语义（核心）

### 2.1 为什么现算 merge-base 足够（探索结论）

- 分支未合并 main：range = 分支独有提交 = 恰好本 change；
- 分支已 merge/rebase main：merge-base 前移，range 收缩为 merge 后自己的提交——**其他 change 经 main 进来的规则编辑永远不在范围内**。存储式 base 与快照方案都做不到这点（快照在合并后会产生假阳性）；
- 零新状态、零迁移成本。

### 2.2 锚点解析顺序

```
merge_base = git merge-base <default_ref> HEAD
default_ref 解析顺序：本地 main → 本地 master → origin/HEAD 指向 → origin/main → origin/master
```

改动点：`crates/llman-core/src/git_utils.rs::resolve_default_branch_ref` 现为 origin 优先——**倒转为本地优先**；新增 `effective_range_base(root) -> Result<String>`（= merge-base(default_ref, HEAD)）作为四个消费方的唯一入口。本地/远端分叉时（local main 领先 origin）打一行 INFO 提示（去重，每进程一次）。

### 2.3 四个消费方改造

| 消费方 | 现状 | 改造 |
|--------|------|------|
| `change/lock_gate.rs::check(root, base_sha, acked)` | `changed_feature_files(base_sha)` + `hashes_at(base_sha)` | 调用方传 `effective_range_base`；worktree 对比逻辑不变 |
| specs landing（`change/specs_landing.rs`） | `git diff --name-only <stored base_sha>...branch -- llmanspec/specs`（r10 口径） | base 改 `effective_range_base`；判定定义同步改条文 |
| `change diff` / finalize 计数提示（`git_native.rs::branch_diff/commit_count_since_base`） | stored base_sha | 改 `effective_range_base` |
| staleness（`spec/staleness.rs` StalenessEvaluator base_ref） | `LLMANSPEC_BASE_REF` env 或 resolve_default_branch_ref | env override 保留（测试用）；默认改 `effective_range_base`。语义：合并进 main 的工作视为已 bless（不再 stale） |

### 2.4 base_sha 字段的去留（兼容矩阵）

- **读取**：frontmatter 中已有 `base_sha`/`baseSha` 继续可解析（旧 change 归档/在途不受影响）；`read_binding` 仍返回它。
- **写入**：`change start`/`attach` 仍写 base_sha（= 绑定时 merge-base，审计/溯源用）。
- **使用**：仓库内所有「拿 base_sha 算 diff 范围」的代码路径清零（grep 审计，见 T1 验收）。
- **checkpoint_sha = base_sha**（r42 语义）：保持不变（都是审计值）。
- 条文改写：r98「base_sha = 与默认分支的 merge-base」补「（审计记录）；一切范围语义 = 现算 merge-base（见 r134/r135 引用）」。

## 3. D2 rules_touched

- frontmatter 新合法字段 `rules_touched`（字符串数组，元素为 req-id，如 `[r131, r141]`）：
  - r118 合法字段集枚举**补上** `rules_touched`（现存漂移：r135 说 `rules_edit_acked` MUST 加入字段集，r118 枚举里没有——一并修复）。
  - 锁定门禁判定：被改动的规则 id ∈ `rules_touched` → 豁免；否则 ERROR。
  - **读取兼容**：`rules_edit_acked: true` ≈ 全量 touched（旧 change 与不想列清单的用户仍可用）；`rules_edit_acked: false`/缺失 + 无 `rules_touched` → 无豁免。
  - 新写入（skill 模板/文档引导）统一用 `rules_touched`；bool 字段文档降级为「兼容保留」。
  - 实现注意：锁定规则改动报告（现状「removed rule (hash)」按哈希报）需映射回 req-id——哈希表构建时保留 id→hash 映射（`validate_locked_scenario` 已收集 rule_req_ids，可复用）。
- r134 条文中「spec-format r24」悬空引用改为 r135（现存漂移一并修）。

## 4. gateChecks（show 聚合门禁，G1/G2）

### 4.1 检查项清单（name 全小写 kebab）

| name | 判定 | hint 示例 |
|------|------|----------|
| `clean-tree` | `git status --porcelain` 为空（start 门禁用） | commit/stash before change start |
| `on-bound-branch` | 当前分支 == binding.branch 且非默认分支 | switch to sdd/<id> |
| `stage-complete` | proposal+design+tasks 齐全（designed+） | 补 design.md/tasks.md |
| `specs-landed` | r10 判定（新锚） | 编辑 live specs 并 commit，或 skip_specs_landing |
| `lock-gate` | r135 判定（新锚 + rules_touched） | 补 rules_touched 或还原规则 |
| `tasks-done` | tasks.md 无未勾项 | 勾选或完成剩余任务 |
| `validate` | `validate <change> --strict` 通过 | 按 validate 输出修复 |

### 4.2 输出契约（token-efficiency 是硬约束）

- `show <change> --json`：新增顶层键 `gateChecks`：`[{"name": str, "pass": bool, "hint": str}]`；hint ≤ 1 行；`pass=true` 时 hint 为空串（省 token）。现有既有键（stage/readyToImplement/specsLanded/…）**不动**（行为冻结键集的追加视为兼容演进，条文落点由实现者核对 cli/cli-experience 既有 show JSON 键归属）。
- text 模式：末尾一段：
  ```
  Gates: 5/7 pass
  ✗ specs-landed: edit live specs on the bound branch and commit (or skip_specs_landing)
  ✗ tasks-done: 2 unchecked tasks
  ```
  **不打印通过项明细**。`readyToImplement` 布尔 = gateChecks 全过（stage=Full 时），与 r1 语义归一。
- `validate` 命令**不改**（保持 issues 口径）；gateChecks 的 validate 项用 `--no-check` 快速口径（避免 show 变慢）。

## 5. 测试计划（seam：既有 CLI 子进程 + 既有 BDD step 库）

Rust 层（sdd_bdd_compat / 单元）：
1. `effective_range_base`：本地 main 优先于 origin（临时 repo：local main 领先 origin/main 时，base == local main 的 merge-base）。
2. lock_gate 新锚：本地 main 领先 origin/main、前一 change 已合入规则编辑时，新 change 零漂移 → 门禁绿（回归实验的直接复现，替换 blanket ack 场景）。
3. merge 后收缩：分支 merge main 后 range 不含 main 侧编辑。
4. rules_touched：列表内豁免、列表外 ERROR、`rules_edit_acked: true` 兼容全豁免。
5. gateChecks：JSON 键集 + hint 非空当且仅当 pass=false；text 只含未过项。

BDD @executable（sdd-workflow.feature，实现者落条文时挂）：
- `gate-accumulation-immune`：两个连续 change（不 push），第二个零规则编辑 → 锁定门禁绿（无需任何 ack）。
- `show-gatechecks-compact`：show text 含 `Gates: n/m pass` 且不含通过项明细。

## 6. 风险

1. staleness 换锚改变 stale 口径（合并后旧漂移不再报）——预期行为（blessed），文档说明。
2. 无 git / detached HEAD 的 `effective_range_base` 失败路径：回退现行为（stored base_sha；再不行=现状错误），fail-open 与现状一致。
3. `rules_touched` 与在途 change 的兼容：此 change 落地时仓库内活跃 change（若有）仍用 bool——兼容层覆盖，无需迁移。
