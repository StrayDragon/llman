# Design：生命周期自表达 v2（无兼容升级）

自包含设计：实现者只需本文件 + proposal + tasks。决策来自 2026-09-11 探索 grilling（proposal.md Open Questions Q1-Q9 + Q4b + Q7c 已全部拍板）。

## 1. 决策记录（摘要）

| # | 决策 | 结论 |
|---|------|------|
| Q1 | 范围 | W1-W4 + @agent + 升级工具，单个 change 一次完成，不拆分 |
| Q2 | start 脏树 | 保持现状报错（不自动 WIP） |
| Q3 | finalize | 默认自动 git commit（`archive(sdd): <id>`）；提供不自动提交选项；无自定义模板 |
| Q4/Q4b | checkpoint | 命令移除；分支提交自由（分段 / finalize 单次都支持），文档说明 |
| Q5 | 阶段 | 四档 `draft → designed → planned → full`（词义修正，不合并） |
| Q6 | specs 检查 | 正向字段 `needs_specs_change`（缺省 true）取代 `skip_specs_landing` |
| Q7/Q7c | 规则确认 | 收尾 y/n；非交互报错点名 + `--yes`（仅 @agent 规则）；`@agent` = 授权 + 审计 |
| Q8 | change new | 不做交互问答 |
| Q9 | 兼容/升级 | 无兼容层；`migrations/v<from>-v<to>/` 标准化升级（脚本 + prompt），SOP 入条文 |

## 2. 阶段四档（Q5）

### 2.1 判定表

`determine_stage` 改为：

```rust
match (has_proposal, has_design, has_tasks) {
    (true, true, true)  => Stage::Full  if attached else Stage::Planned,
    (true, true, false) => Stage::Designed,
    (true, false, _)    => Stage::Draft,   // tasks-without-design 由 check_design_tasks_constraint 报 ERROR
    _                   => Stage::Draft,
}
```

- `attached`（frontmatter 含非空 branch + base_sha）仅把 `planned → full`；绑定本身不改变文件档位（attached 但缺 tasks → 仍 `designed`，`attached=true` 单独暴露）。
- `Stage` 枚举：`Draft | Designed | Planned | Full`；`as_str()` 输出 `draft|designed|planned|full`。
- 旧「designed=三文件齐全」语义删除；`--stage` override（validate）合法值扩为四档。

### 2.2 show/list 输出

- `show <change>` text：`stage: <档位>`（现有行不变，值域变四档）；「距下一档缺什么」由 `gateChecks` 的 `stage-complete` 项 hint 承载（见 2.3），不新增输出行。
- `list`：stage 值域同步；`idleDays` 规则不变（draft/designed/planned 都会被标注停留天数——按「未 full」口径）。
- JSON 键集不变（仅 stage 字符串值变化）。

### 2.3 gateChecks.stage-complete 语义更新

现判定「proposal+design+tasks 三文件存在」不变（planned 是 apply 的最低文件门槛），但 **hint 按当前档位动态**：

| 当前档位 | pass | hint（pass=false 时） |
|----------|------|-----------------------|
| draft（缺 design） | false | `add design.md (current: draft → designed)` |
| designed（缺 tasks） | false | `add tasks.md (current: designed → planned)` |
| planned | true | ""（绑定由 on-bound-branch 管） |
| full | true | "" |

## 3. `needs_specs_change`（Q6，取代 skip_specs_landing）

### 3.1 字段

- proposal frontmatter 合法字段新增 `needs_specs_change`（bool，缺省 true；显式 `false` 才跳过）。
- `skip_specs_landing` 从合法字段集**移除**（出现即未知字段 ERROR）；无兼容读取代码。
- `ProposalFrontmatter.needs_specs_change: bool`（解析缺省 true）。

### 3.2 检查语义（specs landing gate / gateChecks.specs-landed）

```
needs_specs_change == false        → 检查跳过（pass）
needs_specs_change == true（缺省） → 绑定分支 diff（effective_range_base...binding.branch）
                                     是否触及 llmanspec/specs/ 下任意文件
                                     （add / remove / update 任一）→ pass
                                     无改动 → fail，hint 给三条指引：
                                       • edit live specs on the bound branch and commit
                                       • set needs_specs_change: false (no contract edits)
                                       • or use the quick path (llman-sdd-quick)
```

- 「触及」口径：目录级 `llmanspec/specs/`，不限于 `.feature`（Q6 原话：specs add/remove/update 三种都算）。
- `evaluate_specs_landing` 的 `ready_to_implement` 语义更新：`Full ∧ specs-landed(pass)`；`skip_specs_landing` 字段与分支删除；`not_ready_message` 指引更新。
- validate 的 landing WARNING 同步。

## 4. checkpoint 退役（Q4）

### 4.1 命令面

- `change checkpoint` 子命令移除：调用 MUST 非零退出并输出单行 `change checkpoint is removed; use change finalize (single close-out command)`（对齐 r115 `change delta` 模式；无兼容别名）。
- `CheckpointArgs` / `run_checkpoint` / `print_commit_count` 的 checkpoint 调用方删除。

### 4.2 字段与代码清理（无兼容）

- `checkpointed` / `checkpoint_sha` / `checkpointSha` 从 frontmatter 合法字段集移除；`ChangeGitBinding` 删除 `checkpointed` / `checkpoint_sha` 字段；`read_binding` / `write_binding` 不再处理。
- `enforce_bdd_archive_gates_inner` 的 `checkpointed` 前置检查删除（`archive` fallback 命令不再要求存档字段）。
- finalize 不再写 checkpoint 字段（见 §5 幂等替代）。
- r137「finalize 与 checkpoint 展示 commitCount」→ 仅 finalize；r42（sdd-workflow checkpoint_sha 语义核对条）内容重写（见 tasks 条文清单）。

## 5. finalize 自动提交（Q3 + Q4b）

### 5.1 新流程

```
finalize <id> [--no-check] [--yes] [--no-commit]
  1. 门禁（attach / branch / 非默认 / 无遗留 delta）
  2. validate（live strict + change，--no-check 跳过）
  3. lock-gate：未声明的锁定规则改动 →
       交互: 列出改动（req-id + 摘要）→ y/n → y 写回 rules_touched（+ agent_acked）
       --yes: 仅对带 @agent 的规则自动写回；未标记规则报错列出
       非交互无 --yes: 报错点名 + 指引
  4. print commit count（现算锚）
  5. ff-merge 到默认分支
  6. docs-only archive rename（changes/<id> → changes/archive/<date>-<id>）
  7. ★ 新增：git add -A && git commit -m "archive(sdd): <change-id>"
       - 一次提交收尾：实现 diff（若未提交）+ frontmatter + archive rename
       - --no-commit: 跳过第 7 步（工作区留脏，调用方自行 commit；skill 文档说明）
  8. 输出结果（含 commit sha 或 --no-commit 时的手动指引）
```

### 5.2 语义与失败

- **message 固定**：`archive(sdd): <change-id>`（无模板配置；用户可 `git commit --amend` 自行改）。
- **提交失败**（hook 拒绝等）：非零退出 + 保留现场（rename/merge 不回滚，与 ff-merge 失败降级一致）+ 提示用 `--no-commit` 或手动 `git commit`。
- **幂等**：重试时若 change 目录已不在 `changes/` 且出现在 `changes/archive/<date>-<id>` → 打印「already finalized」并成功退出（不再依赖 checkpointed 字段）。若 rename 完成但 commit 失败，重试时先检查 rename 状态再补 commit。
- **Q4b 文档**：apply/archive skill 与 AGENTS.md 说明——分支上提交自由，分段 commit 或 finalize 单次收尾均可；checkpoint 不再需要。

## 6. 锁定规则收尾确认 + `--yes` + `@agent`（Q7/Q7c）

### 6.1 触发与交互

- 触发：lock-gate 检测到「被改动的锁定规则 req-id ∉ rules_touched（也 ∉ agent_acked）」。
- 交互模式（TTY 且无 `--no-interactive`）：一次列出全部未声明改动（`<feature>: <kind> rule @req:<id>`）→ 单个 y/n 确认：
  - y → 把全部检测到的 req-id 追加写入 `rules_touched`（frontmatter 重写）→ 继续；
  - n → 报错退出（错误信息含同一清单）。
- 非交互：报错输出 = 现状按 req-id 点名 + 两条可复制指引：
  - `add rules_touched: [<req-id>,...] to proposal.md frontmatter`
  - `or pass --yes to acknowledge @agent-marked rules`
- `--yes`：把「检测到的改动中、其规则带 `@agent` 标记」的 req-id 自动写入 `rules_touched`（并记入 `agent_acked`）后继续；未带 `@agent` 的改动仍报错列出（混合情形只放行前者）。

### 6.2 `@agent` 标记（语义 C：授权 + 审计）

- **语法**：`.feature` 场景 tag，`@agent` 必须与 `@human` 同场景（`@req:rX @human @agent`）。
  - 单独 `@agent`（无 `@human`）→ validate ERROR（`@agent requires @human`）。
  - `@agent` 场景仍属 Locked tier（锁定哈希 = id+name+description+steps（r135），tag 不入哈希：仅给规则**加 `@agent` 标记本身不触发锁定门禁**；改动规则文本仍走同一确认路径）。
- **授权**：`--yes` 仅对此类规则的改动生效（§6.1）。
- **审计**：新增可选 frontmatter 字段 `agent_acked: [<req-id>...]`（合法字段集新增；仅 agent 写回时填入）。呈现：
  - `llman sdd review` 的 locked 信号 detail 追加「N rule(s) ack'd by agent」；
  - `change diff` 人读输出在文件列表后附 `agent-acked rules: @req:...` 行（无则不打印）。
- **tag 语法学**：spec-format r132 保留字汇新增 `@agent`（授权+审计语义，具体校验见上）。

## 7. 升级工具标准化（Q9）

### 7.1 目录结构（新顶层 `migrations/`）

```
migrations/
  README.md                      # SOP：破坏性发布 MUST 建 v<from>-v<to>/ 目录；
                                 # 目录要素=README(升级 prompt)+一次性脚本；脚本默认 dry-run
  v0.0.75-v0.0.76/
    README.md                    # 升级 prompt（用户/agent 执行指令）
    upgrade_lifecycle_v2.py      # 一次性升级脚本
```

### 7.2 升级脚本行为

- 默认 **dry-run**（打印将改内容），`--apply` 执行；`--root <path>` 可选（默认当前目录）。
- 扫描 `llmanspec/changes/*/proposal.md`（**跳过 `changes/archive/`**，历史只读）：
  - `skip_specs_landing: true` → 删字段，写 `needs_specs_change: false`；
  - `skip_specs_landing: false` → 删字段；
  - `checkpointed` / `checkpoint_sha` / `checkpointSha` → 删；
  - `baseSha` → `base_sha`（冲突时报错不覆盖）；
  - `rules_edit_acked` → 删除；若为真值且无 `rules_touched`，打印**人工处理清单**（该 change 需人工确定被改动的 req-id 后写 `rules_touched`）。
- 输出摘要：每文件变更行 + 未自动处理项 + 收尾提示（跑 `llman sdd validate --all --strict`）。
- 幂等：重复执行无内容可改时报告 no-op。

### 7.3 SOP 条文落点

sdd-workflow 新增要求（见 tasks 条文清单）：破坏性合约变更 MUST 在 `migrations/v<from>-v<to>/` 提供升级 README（prompt）+ 脚本；脚本 MUST 默认 dry-run、MUST 跳过 `changes/archive/`；发布说明 MUST 指向该目录。

## 8. 无兼容矩阵（Q9）

| 旧事物 | 处理 | 拒绝方式 |
|--------|------|---------|
| `skip_specs_landing` | 升级脚本转 `needs_specs_change: false` | 未知字段 ERROR |
| `checkpointed`/`checkpoint_sha`/`checkpointSha` | 升级脚本删除 | 未知字段 ERROR |
| `rules_edit_acked` | 升级脚本删除（true 转人工清单） | 未知字段 ERROR |
| `baseSha` | 升级脚本归一 `base_sha` | 未知字段 ERROR |
| `change checkpoint` | 命令移除 | 单行移除提示 |
| 三态 stage 输出 | 直接四档（stage 是推导量，无数据迁移） | — |
| `changes/archive/**` | 不动（validate 免检） | — |

## 9. 失败语义

- 所有新检查失败均为**单行 token 友好**错误 + 可复制指引，不打印堆栈。
- finalize 自动提交失败不回滚 merge/rename，保留现场并给出 `--no-commit`/手动 commit 指引。
- 升级脚本非 `--apply` 时 MUST NOT 写任何文件。

## 10. 测试计划（seam S1/S2/S3）

- **S1 既有回归网**：`cargo test --features bdd`（全场景）+ `validate --all --strict` 全绿（四档/字段迁移后的既有 fixture 需同步适配）。
- **S2 新行为**（@executable 场景 + Rust 测试，走既有 CLI harness / BDD step 库）：
  1. 四档判定：draft / designed（+design）/ planned（+tasks）/ full（+绑定）各一 fixture，断言 `show --output json` 的 stage；
  2. `checkpoint` 移除：调用非零 + 单行提示；
  3. finalize 自动提交：临时 repo 跑 finalize → 断言新 commit message == `archive(sdd): <id>` 且树干净；`--no-commit` → 无新 commit、树留脏；
  4. `needs_specs_change`：true 无 specs diff → 报错+指引；false → 放行；有 specs 任意改动（add/remove/update）→ 放行；
  5. `--yes` 收窄：@agent 规则改动 → 自动写 rules_touched/agent_acked 通过；无 @agent → 仍报错；混合 → 仅前者放行；
  6. `@agent` 解析：合法组合通过；单独 @agent → ERROR；tag 增删触发锁定哈希（门禁路径）；
  7. 收尾 y/n 交互：**不测**（交互流程不要求自动化），测其非交互分支。
- **S3 升级工具验证**：fixture change 含全部旧字段（`skip_specs_landing: true`, `checkpointed`, `checkpoint_sha`, `checkpointSha`, `baseSha`, `rules_edit_acked: true`）→ dry-run 报告正确 → `--apply` → 断言字段转换/删除正确、人工清单输出、`validate --all --strict` 全绿（人工清单项处理后）。

## 11. 风险

1. 无兼容升级会拒绝所有未迁移的旧项目——由 `migrations/v0.0.75-v0.0.76/` 升级 prompt + 脚本覆盖；发布说明须显著提示。
2. finalize 自动提交与用户的 pre-commit hook 可能冲突 → `--no-commit` 逃生 + 失败保留现场。
3. `@agent` 授权范围的误用（人类一次授权后 agent 长期代确认）→ 审计字段 `agent_acked` + review 浮现，人类可定期复查标记清单。
4. BDD 慢入口（Q4 已记录）不在本 change 解决；checkpoint 退役仅减少一个慢路径。
