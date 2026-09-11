---
depends_on: [git-native-v2]
---

# 生命周期自表达：draft 落地足够信息，配置趋零，cli 开箱即用

## Why

现行生命周期要求使用者预知并手工维护多类「元配置」：proposal frontmatter 的 `skip_specs_landing`、`rules_edit_acked`（v2 后为 `rules_touched`）、stage 三态的推进命令（change start 前 must designed）。这类配置的本质是**让工具在事后能推断意图**——但意图在事件发生时就已存在，理应由工具在事件时刻捕获，而不是让使用者预先声明。需求方方向（2026-09-11 探索拍板）：把足够的信息在 draft 阶段就落地保存；尽可能减少 `rules_edit_acked` 类配置；cli 开箱即用。

## Open Questions（explore grilling 决策记录，2026-09-11）

- **Q1 范围（已决）**：W1-W4 四项**单个 change** 一次完成，不拆分。
- **Q2 start 脏树（已决）**：**不自动 WIP，保持现状报错**。工作区不干净时 `change start` 非零退出并输出现状的一行提示（dirty tree: N uncommitted files; commit/stash before change start），不加多余提示，由用户自行决定如何处理。原因：防止未提交 live specs 改动随分支切换「偷渡」进 Specs landing（r1 默认分支不得接收未实现合约）；防止无关脏改动被算入本 change 范围。→ W3 的「start 遇脏树自动打 WIP commit」子项**否决**。
- **Q3 finalize 自动提交（已决）**：`change finalize` 收尾**自动 git commit**（默认开）；提供**不自动提交的 CLI 选项**并在 skill 文档中说明该选项，避免特定情况（CI/脚本/用户自定义历史）出问题。提交说明用**简单固定式** `archive(sdd): <change-id>`；暂不提供用户自定义模板配置（项目级/全局级配置的想法另有 draft 提案记录）。`--amend` 由用户在自动提交后自行运行 `git commit --amend` 修改说明，finalize 不提供 --amend 参数。
- **Q4 checkpoint 退役（已决）**：**移除** `change checkpoint` 命令（r115 change delta 先例：调用即报错提示改用 finalize）；`checkpointed`/`checkpoint_sha`/`checkpointSha` 字段**直接移除**（无兼容读取；升级工具删除字段，见 Q9）。动机之一：「llmanspec 若干流程默认走 bdd run_command（exec test）入口导致开发慢」——checkpoint 每次跑 validate 全检（含 BDD runner）是慢路径之一，退役后缓解；但 `validate --specs` / `finalize` 的 bdd check 仍在，该慢入口整体问题单列**待定**（不在本 change 强制范围）。
- **Q4b 分支写代码的自由度（已决）**：默认提示词 / skills 模板（apply/归档 skill、AGENTS.md commit 策略）新增说明：**在 change 分支上写代码时提交是自由的**——可按用户要求或自行按惯用方式**分段 commit**（大 change 分开提交对 review 友好），也可保持工作区不提交、最后走 finalize **单次 commit** 收尾；checkpoint 退役后分段提交不再需要任何存档命令配合，finalize 对两种情况都原生支持（不要求干净树，多 commit 直接 ff-merge）。
- **Q5 阶段语义（已决）**：保持可解释的多档结构，**不合并**；改为四档且每档名与完成物对齐：`draft`（仅 proposal）→ `designed`（proposal + design；**词义修正**：design 存在即达，不再等 tasks）→ `planned`（新增；proposal + design + tasks 齐全，原 designed 档更名）→ `full`（+ 绑定，不变）。升级仍由文件生长自然驱动（不新增推进命令）；`show` MUST 显示当前档位与距下一档缺什么。依赖链不变：proposal ← design ← tasks（「有 tasks 无 design」仍为 ERROR）。兼容面：`--stage` override / r93 条文 / gateChecks 的 stage 判定 / 归档旧数据（只读不动）。
- **Q6 specs 检查声明（已决）**：不采用自动推导；改用 proposal frontmatter 的**正向字段 `needs_specs_change`（缺省 true）**控制「本分支是否改动过 specs」的检查，不引入新 config 文件。语义：`true`（缺省）→ CLI 执行检查：绑定分支 diff 只要触及 `llmanspec/specs/`（文件 add/remove/update 任一）即校验通过；无改动 → 报错并给出指引（改 specs / 声明 `false` / 走 quick 路径）。`false` → CLI 完全跳过该检查（无需 spec 的 change 走 full 流程也能收尾）。旧字段 `skip_specs_landing` **直接移除**（无兼容读取）；升级工具将 `true` 转换为 `needs_specs_change: false`（见 Q9）。理由：大部分 change 需要改 spec，缺省检查有效；quick 与 full 路径不可避免会混用，用显式声明把意图保留在 change 自己的元信息里（proposal frontmatter，SSOT）。
- **Q7 rules_touched 确认时机（已决）**：不再要求开工前预写，改为**收尾确认**：finalize / validate 时门禁发现锁定规则改动且未声明 → 交互模式（有人看着）：**一次列出全部改动（按 req-id + 摘要）、一个 y/n 确认** → y：自动写回 `rules_touched` 到 proposal frontmatter 后继续；n：报错停下。**非交互模式（CI/脚本/agent）**保持「要求预写、报错」并同时支持两条路（A 保底 + B 快捷）：
  - **A**：报错输出按 req-id 点名（git-native-v2 已实现）+ 直接给出可复制粘贴的修复指引（`add rules_touched: [r…] … or pass --yes`），agent 可自行完成「读报错→写 frontmatter→重跑」闭环；
  - **B**：`finalize` / `validate` 新增显式 `--yes`（对齐 `project migrate --yes` 惯例）：表示「我确认：把检测到的规则改动自动写入 rules_touched 后继续」；**必须显式传参**，不是默认行为，锁定门禁语义保留。
- **Q8 change new 交互问答（已决）**：**不实现**，`change new` 保留原样（现有 `--from` 推导 id 不变）。理由：proposal 的格式本身不是固定化的，硬编码问答会把格式绑死；「draft 自足」不再以此实现。
- **Q9 放弃兼容层 + 标准化升级流程（已决）**：
  - **无兼容**：`skip_specs_landing` / `checkpointed` / `checkpoint_sha` / `checkpointSha` / `rules_edit_acked` / `baseSha` 别名全部移除、无兼容读取；出现即 validate 未知字段 ERROR（干净一步到位）。
  - **升级工具标准化（SOP，今后每次破坏性发布照此办理）**：仓库新增顶层目录 `migrations/`，按版本区间建目录 `v<from>-v<to>`（本 change = `migrations/v0.0.75-v0.0.76/`），内含：
    - `README.md`：升级 prompt（给用户/agent 的执行指令：dry-run 报告 → apply → 人工处理项 → `validate --all` 验证）；
    - 一次性升级脚本（Python；默认 dry-run，`--apply` 才写；**跳过 `changes/archive/`**——历史只读；覆盖 active changes 的 frontmatter 字段迁移；`rules_edit_acked: true` 无法自动确定规则编号 → 打印人工处理清单）。
  - SOP 写入规范条文（破坏性变更 MUST 提供 `migrations/v<from>-v<to>/` 升级指引与脚本）。
  - **seam S3 相应变更**：兼容矩阵 → 升级工具验证（fixture 造含全部旧字段的 change → dry-run 报告 → `--apply` → 断言字段转换正确 + `validate --all --strict` 全绿）。
- **Q7c @agent 标记（已决，纳入本 change）**：语义 = **C（授权 + 审计）**：
  - **授权层**：人类在 `.feature` 的 `@human` 场景上打 `@agent`（与 `@human` 并列）→ 该规则的确认托管给 agent；
  - **确认层**：`--yes` **只对带 `@agent` 的规则生效**（自动写 rules_touched）；未标记规则 `--yes` 无效，保持「必须人类确认」（交互 y/n 或预写 rules_touched），非交互报错 + 指引；
  - **审计层**：被 agent 确认/触碰的规则在 review / diff 输出中浮现（供人类重点审查）。呈现机制留 design 阶段细化；原则：`@agent` 场景 tag 只表达「授权」单一含义，审计痕迹不污染长期规则文件的语义（不靠反复改写 .feature 记历史）；
  - **规范面**：spec-format tag 语法学新增 `@agent`；锁定门禁（r135）确认路径按标记收窄。

## What Changes

1. **阶段四档（词义修正）**：`stage` 判定改为 `draft`（仅 proposal）→ `designed`（proposal + design）→ `planned`（proposal + design + tasks）→ `full`（+ 绑定）。`designed` 不再要求 tasks 同时存在；「有 tasks 无 design」仍为 ERROR。`show` MUST 显示当前档位与距下一档缺什么；不再新增阶段推进命令（档位仍由文件生长 + 绑定驱动）。
2. **checkpoint 退役 + finalize 收口**：移除 `change checkpoint` 命令（调用报错指引用 finalize）；`finalize` 默认**自动 git commit**（固定说明 `archive(sdd): <change-id>`），提供不自动提交的 CLI 选项，供 CI/脚本/特殊场景使用；文档/skills 明确「change 分支上提交自由：分段 commit 或 finalize 单次提交都支持」。
3. **`needs_specs_change` 取代 `skip_specs_landing`**：proposal frontmatter 新增正向字段（缺省 `true`）：`true` → 执行「绑定分支是否改动过 `llmanspec/specs/`（add/remove/update 任一）」检查，无改动则报错 + 指引；`false` → 完全跳过该检查。
4. **锁定规则收尾确认 + `@agent` 标记**：不再要求开工前预写 `rules_touched`；finalize/validate 发现锁定规则改动且未声明时：交互模式一次列出全部改动（req-id）并 y/n 确认（确认后自动写回 `rules_touched`）；非交互模式报错点名 req-id + 给可复制修复指引，并提供显式 `--yes`（仅对带 `@agent` 标记的规则生效）。`@agent`（新增 tag，与 `@human` 并列）= 授权（该规则确认托管给 agent）+ 审计（被 agent 确认/触碰的规则在 review/diff 输出中浮现）。
5. **标准化升级流程（SOP）**：破坏性合约变更 MUST 提供 `migrations/v<from>-v<to>/` 升级目录（README 升级 prompt + 一次性脚本，脚本默认 dry-run、`--apply` 执行、跳过 `changes/archive/`）；本 change 提供 `migrations/v0.0.75-v0.0.76/`。本条写入规范条文，今后照此办理。
6. **无兼容层**：`skip_specs_landing` / `checkpointed` / `checkpoint_sha` / `checkpointSha` / `rules_edit_acked` / `baseSha` 别名全部移除、无兼容读取（出现即 validate ERROR）；旧状态由升级工具一次性清理。
7. **文档/技能同步**：AGENTS.md（commit 策略 / Locked rules 口径 / 阶段守卫）、SDD 模板（feature-contract / validation-hints / stage-guard / git-native-flow）、skills（apply/archive/propose 等）。

## Capabilities / Impact

- `sdd-workflow`（r93 三态→四档、r1 landing 判定、r111 start 措辞、r124 字段集、r137 checkpoint 引用、r42 finalize 语义、新增升级 SOP 条文）
- `sdd-bdd-mode-compat`（r42 finalize / checkpoint 相关条文、`change checkpoint --no-interactive` 场景）
- `spec-format`（r132 tag 语法学新增 `@agent`、r135 锁定门禁确认路径收窄）
- `cli-experience`（checkpoint 移除、finalize 新选项、show 档位输出）

> 依赖 `git-native-v2` 已归档（范围换锚与 rules_touched 落地后减配置，本次直接在其上做无兼容升级）。
> 决策记录见上方 Open Questions（Q1-Q9 + Q4b + Q7c），正式化实现以 design.md / tasks.md 为准。
