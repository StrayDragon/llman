# Design: add-sdd-pipeline-guards

## 问题细化（盘点证据 → 合约动作映射）

| # | 问题（证据） | 细化结论 | 动作 |
|---|---|---|---|
| 1 | review 三时点断链：AGENTS.md 强制、CLI 存在、`.agents/skills/**` grep `sdd review` 零命中 | agent 执行面是 skill 文件；检查点必须写进模板才生效 | 新规则（structured-prompts）+ 模板接线 |
| 2 | commit 纪律靠自觉：src-cleanup-pre-split 14+ commit（add-apply-commit-discipline 提案自述）；8f3be4f 只有文字 | 缺可观测信号；agent 无法在循环中感知「已多 commit」 | 新规则（workflow）+ diff commitCount + finalize/checkpoint 提示 |
| 3 | 悬空引用：apply 无条件引用 arch-review；r96 本就要求按 extra_skills 条件化 | 这是 r96 既有合约的违规，不是新规则；修复 = 合规化 + 本项目启用 | 模板 fallback（复用 r96）+ config extra_skills |
| 4 | valid_scope 手动循环：propose/apply preflight 各自要求逐个核对 30 spec | 机械检查下沉 CLI；agent 只读报告 | 新规则（workflow）+ validate 检查 |
| 5 | context 自动 rebuild（P2 最初项） | **r97 已覆盖且已实现**；skill 文案是 api_error 兜底，保留 | 无（盘点修正） |
| 6 | draft 滞留不可见：add-meta-skill-dynamic-prompts 一个月未 triage；list --json 已有 lastModified 但缺人读口径 | 口径同源 lastModified，零新数据源 | 新规则（workflow）+ idleDays |
| 7 | verify 编号断档 1→3（模板源即如此）；双轴表行与 r103 口径漂移；apply 8 轮 vs apply-cycle 3 次；graph/quick 带无关校验修复单元 | 单元插位破坏宿主编号是可断言的回归面；表行是文档漂移 | 新规则（template-units）+ 组装修复 + check 断言 + AGENTS.md 表行 |
| 8 | quick-path commit trailer（盘点可选项） | 收益低，超出「一致性小修」边界 | 不做，留 backlog |

## 决策

- **D1 规则归属**：CLI 行为合约归 `sdd-workflow`（生命周期与命令面规则的既有家）；
  skill 模板内容要求归 `sdd-structured-skill-prompts`（r32/r65/r96 同类）；
  模板组装机制归 `sdd-template-units-and-jinja`（r33/r66 同类）。
- **D2 commitCount 阈值**：> 1 即提示、不阻断、零新 config 字段。单 commit 纪律下
  base_sha 之后本应 0（实现留工作区）或 1（WIP blocker）；2+ 即已违反纪律，
  无需可配置阈值（避免 schema 变更，对齐 review r6 零配置哲学）。
- **D3 idleDays 口径**：proposal.md mtime（与 list --json 既有 `lastModified`
  同源），`now - mtime` 向下取整 UTC 天；文本仅对 draft/designed 追加标注，
  full 及以后不标（避免归档前噪声）。
- **D4 valid_scope 检查语义**：strict → ERROR（非零退出），默认 → WARNING；
  消息 MUST 列出缺失路径。不阻塞 spec 格式校验的其它项。
- **D5 双轴表行**：改 AGENTS.md 文档对齐 r103（r103 定义为 MUST，always-on），
  不给 verify 模板加触发 gate——放宽到「触发式」属于合约弱化，反向对齐才对。
- **D6 arch-review 双管齐下**：本项目 config.yaml 启用（agent 真能调用）+
  apply 模板补 fallback 措辞（其他未启用项目合规，r96）。
- **D7 编号连续性**：修复方式 = 阶段守卫单元不再插入宿主有序列表内部（移到
  步骤列表之前/之后），而非改编号写法；`just check-sdd-templates` 加断言防回归。

## 测试接缝（seam，与用户确认过）

- **CLI 子进程**：`llman sdd validate --specs --strict` / `llman sdd change diff <id> --json`
  / `llman sdd list --json`，由 `tests/bdd_steps.rs` 既有泛化步骤驱动；
  新增 1 个 fixture Given（含失效 scope 路径的 spec 项目）+ JSON 数值路径步骤
  支持数组段（`changes.0.idleDays`）。
- **模板门禁**：`just check-sdd-templates`（版本头 + locale parity + 新增编号
  连续性断言）；模板内容正确性由 verify 双轴人工审查兜底。

## 非目标

- 不动共享单元结构与 token 布局（属 `add-meta-skill-dynamic-prompts` 草案）。
- 不改 review 五类信号（r6 零配置 v1 边界）。
- 不做 quick-path commit trailer。
- 不改既有 @human 规则（避免锁定门禁；仅纯新增）。
