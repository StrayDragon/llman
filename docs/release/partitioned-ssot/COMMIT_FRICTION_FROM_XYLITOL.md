# BDD-on commit friction（from xylitol, 2026-07-18）

给 **llman** 仓其他 agent：这是 consumer 真实分支上的操作摩擦，不是抽象吐槽。

## 证据（xylitol `feat/tui-dev` vs origin）

- 未 push ~**50** commits。
- 其中 ~**26** 为 `chore(sdd)` / `docs(sdd)`：`draft` / `checkpoint` / `archive`。
- 单条完整闭环在 skills 要求下至少：

  ```text
  feat/fix 实现
  → chore(sdd): checkpoint <id>
  → chore(sdd): archive <id>
  ```

  常再加 `draft` commit（roadmap 一批意向入库，属合法实践）→ 流程 commit 占比仍高。

## 根因

`checkpoint` 要求干净树且会改 `proposal.md`；`archive` 又要求干净树。
→ 元数据与 rename **必须**各占一次 commit。`improve-partitioned-ssot-agent-friction`
已文档化该时序，但未提供合并收尾命令。

## 请求落地

见 draft change：

`llmanspec/changes/improve-bdd-on-finalize-and-commit-hygiene/proposal.md`

优先：

1. `llman sdd change finalize <id>`（checkpoint + archive 同进程，一次留给 git commit）
2. 或放宽：archive 允许「仅本 change proposal checkpoint 字段脏」

次要：skills 写明可批量 finalize；**不要**禁止单独/批量 draft commit（consumer 可从 roadmap 一次入库多草案）。

## Consumer 已做的习惯（xylitol）

`xylitol/llmanspec/AGENTS.md` →「提交卫生（SHOULD）」：批量收尾、`chore(sdd)` 与产品 commit 分离；draft 可独提/批提。

## 不要做

- 不要为了少 commit 而削弱 `checkpoint_sha` 审计。
- 不要把 Partitioned SSOT 退回 change-scoped delta。
