---
depends_on: []
---

# SDD Pipeline 护栏：review 接线、commit 信号与 CLI 自动化下沉

## Why

对 10 个托管 skill 与近 620 个 commit 的盘点（决策记录见 design.md）发现：工作流
self-hosting 循环健康，但存在 4 类结构性缺口，均为「agent 行为靠纸面约束、缺机械
信号」或「skill 间引用断链」：

1. **review 检查点断链**：根 AGENTS.md 强制三个时点运行 `llman sdd review`
   （apply 批次后 / verify→finalize 前 / archive 前），CLI 命令已存在
   （`crates/llman-sdd/src/sdd/review.rs`），但 `.agents/skills/**` 全目录
   grep `sdd review` 零命中——agent 实际执行的 skill 文件从未引导运行它。
2. **commit 纪律缺机械信号**：`add-apply-commit-discipline` 提案自述
   `src-cleanup-pre-split` 在 main 留下 14+ commit（逐 task 提交）；
   `8f3be4f` 已把 Commit 策略写进 apply 模板，但只是文字约定，没有任何
   可观测信号阻止复发。
3. **skill 引用了未启用的兄弟 skill**：apply 模板无条件引用
   `llman-sdd-arch-review`（本项目 config 未启用），违反既有合约 r96
   「optional skill 引用 MUST 仅在 extra_skills 包含时出现，否则给出
   不依赖该 skill 的替代指引」；propose 对 continue 的引用已有 fallback 措辞。
4. **可自动化的手动循环下沉不足**：propose/apply preflight 都要求 agent
   「逐个核对全部 spec 的 valid_scope 路径存在性」（30 个 spec 的手动循环）；
   draft 草案无停留时长可见性（`add-meta-skill-dynamic-prompts` 已滞留一个月）。

盘点中的两处修正（细化后剔除，不做）：

- `context` stale/missing 自动 rebuild **已由 r97 覆盖并实现**，skill 文案的
  「不可用时 rebuild 后重试」仅为 api_error 兜底，无需新 CLI 行为。
- quick-path commit trailer 约定：收益低，留 backlog 不进本 change。

## What Changes

**CLI 行为（3 项，均走既有 `llman sdd` 子命令 seam + BDD 场景）**

- `validate`（--specs / --all / --strict 路径）MUST 校验每个 spec 的
  `valid_scope` 路径在磁盘存在：缺失路径按 strict 报 ERROR（非零退出），
  默认模式报 WARNING，消息含缺失路径清单。
- `change diff <id>` MUST 报告自 base_sha 以来的 commit 数（新增 `--json`
  含 `commitCount` 数值；人读输出含计数行）；`change finalize` /
  `change checkpoint` 在计数 > 1 时 MUST 打印不阻断的提示（语义收敛建议）。
  零新 config 字段（对齐 review r6 的零配置哲学）。
- `list` 文本人读输出对 draft/designed change MUST 追加停留天数标注；
  `list --json` 每个 change MUST 含 `idleDays` 数值（口径与既有
  `lastModified` 同源：proposal.md mtime，UTC 整数天）。

**模板与文档（5 项，经 `init --update` 双 locale resync，`just check-sdd-templates` 门禁）**

- review 三时点接线：apply（每 task 批次后）/ verify（全绿后 finalize 前）/
  archive（逐个归档前）模板 MUST 引导运行 `llman sdd review`，非零退出 =
  CRITICAL → 停止修复后再继续。
- 「校验修复」单元按职责注入：从 graph/quick（永不编辑 live specs）摘除，
  保留在 propose/apply/verify/archive/specs-compact。
- arch-review 引用合规：apply 模板补不依赖该 skill 的替代指引（r96）；
  本项目 config.yaml `extra_skills` 启用 `llman-sdd-arch-review` 并 resync。
- 一致性：apply-cycle 重试预算与 apply 统一表述；verify 模板组装修复步骤
  编号断档（阶段守卫单元插位吞掉 step 2）；AGENTS.md 增强表「双轴审查」行
  对齐 r103 实际语义（always-on，非触发式）。
- `just check-sdd-templates` 新增渲染产物步骤编号连续性断言（防回归）。

**Specs landing（3 个 capability 各加新 @human 规则 + @executable 场景，纯新增不改既有规则）**

- `sdd-workflow`：valid_scope 校验、commitCount 信号、idleDays 三条新规则。
- `sdd-structured-skill-prompts`：review 三时点接线、校验修复单元职责注入两条新规则。
- `sdd-template-units-and-jinja`：渲染产物宿主编号连续性一条新规则。

## Capabilities

- `sdd-workflow`：3 条新 @human 规则（valid_scope 检查 / commitCount / idleDays）
  + 3 个 @executable 场景。
- `sdd-structured-skill-prompts`：2 条新 @human 规则（review 接线 / 单元职责注入）。
- `sdd-template-units-and-jinja`：1 条新 @human 规则（编号连续性）。

## Impact

- 受影响范围：`crates/llman-sdd`（validate/diff/finalize/list）、
  `templates/sdd/{en,zh-Hans}/skills/**`、`tests/bdd_steps.rs`（新 fixture Given
  + JSON 数值路径步骤支持数组段）、本项目 `config.yaml`（extra_skills 一行）、
  根 `AGENTS.md`（增强表一行）、3 个 live `.feature`。
- 行为变化：validate --all 可能因既有 spec 的失效 scope 路径出现新失败
  （预检已确认当前 30 spec 无缺失，风险低）；list 文本新增标注；
  diff 新增 --json。模板渲染产物全部经 init --update 刷新。
- 锁定规则：本 change 仅新增场景，不改既有 @human 规则，无需 rules_edit_acked。
- 与 `add-meta-skill-dynamic-prompts` 草案无文件交集（该草案关注模板 token
  成本，本 change 不动共享单元结构）。
