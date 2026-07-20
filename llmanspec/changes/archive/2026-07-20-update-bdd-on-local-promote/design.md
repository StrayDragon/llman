# Design

## 核心决策

### 1. 不引入 `promote.style` 配置（本轮明确不做）

**背景**：FR 提到可加 `llmanspec/config.yaml` 的 `sdd.promote.style: local|pr`，按项目条件渲染不同收尾段。

**决策**：**不做**。理由：
- 现状 SDD skill 模板是**静态渲染**（`llman sdd init --update` 时按 `bdd_enabled` 等变量一次性展开 MiniJinja），安装后就是固定的 SKILL.md 文件。
- 加 `promote.style` 需要在渲染时读 config 并条件分支，但已安装的 `.agents/skills/**` 副本不会随用户后续改 config 自动重渲染——只有再跑 `init --update` 才生效，这对「下游用户改个配置就生效」的体验是误导。
- 真正干净的实现需要**元 skill / 动态 skill**机制（skill 在运行时读 config 决定叙事），这是更大的改动，超出本轮范围。
- 现阶段最务实的做法：**统一默认叙事为本地 merge**，把 push/PR 降为「用户明确要求时的可选步骤」。这对个人仓是正确的默认，对团队仓也不损失（团队仓用户会明确说要开 PR，agent 仍可执行）。

### 2. 合约归属：扩 r65 + 新增 r98

**为什么不全加新 requirement**：
- r65 既有合约「propose 与 archive 技能对齐 Git-native」已经说「再 Git merge」——本次只是**让实现回归合约**（模板写成「Git/PR merge」是偏离）。直接在 r65 补一句「默认叙事为本地 merge，MUST NOT 默认导向 push/PR」语义内聚，避免合约碎片化。
- r98 是**新行为**（apply-cycle 加步骤 + finalize 提示 + validate next-steps 去误导），r65 不覆盖这些，需新 requirement。

**为什么 r98 放 `sdd-structured-skill-prompts` 而非 `sdd-bdd-mode-compat`**：
- r98 的三条行为里，apply-cycle 技能措辞是 skill 模板合约（`sdd-structured-skill-prompts` 主场）。
- finalize/validate 的 CLI 输出虽然属于 `sdd-bdd-mode-compat` 领域，但 r98 的**统一主题**是「BDD-on 收尾提示不默认导向 PR/push」——把强相关的三条放一个 requirement 里，便于 agent 一次性理解完整意图，也减少跨 spec 跳转。
- `sdd-bdd-mode-compat` 的 r94（finalize 单 commit）等已有 requirement 不受影响；r98 是补充而非覆盖。

### 3. finalize 成功提示走 `t!` 本地化键

**现状**：`finalize.rs:112-115` 的成功 `println!` 是**硬编码英文**，与 `archive` 走 `t!()` 不一致——这违反 `cli-experience` r43（「运行时提示 MUST 优先用 `t!` 本地化键」）。

**决策**：本次一并修复——新增 locale key（如 `sdd.change.finalize_success_next`），finalize 成功后追加一行 next-step 提示。既满足 r98（本地 merge 提示），又顺带合规 r43。

**文案草案**（英文，locale 固定 en）：
```
Next: commit on the feature branch, then merge into <default> locally
      (push / hosting PR optional — only if explicitly required).
```
`<default>` 在运行时从 git 读取默认分支名（与 finalize 已有的 branch 读取逻辑复用）。

### 4. validate next-steps 按 BDD 模式分支

**现状**：`print_next_steps` 对 `ItemType::Change` 无条件打印 `change_step_1`（「Ensure change has deltas in specs/」）——这是 BDD-off delta 流程的残留，BDD-on 下 change 内 TOON delta 被显式忽略（见 `validate_change_full` 的 BDD-on 分支），诱导用户写 delta 是误导。

**决策**：`print_next_steps` 检测 `config` 是否含 `bdd:` 段（需传入或从上下文获取），BDD-on 下改打新 key `change_step_1_bdd_on`（指向 live specs + attach/finalize），BDD-off 下保留原 `change_step_1`。

**实现注意**：`print_next_steps` 目前签名是 `(item_type, issues)`，不含 config 信息。需要扩展签名传入 `bdd_enabled: bool`（或 `&Config`），调用方 `print_single_report` 从 validate 主流程获取。这是一处小的函数签名调整，不破坏现有调用（调用点集中）。

## 可执行测试边界（apply 阶段定夺）

`local-promote-hints.feature` 的三个场景：
1. **apply-cycle 技能含本地 merge 步骤**：文档型（检查渲染产物文本），无 `@executable`，与 r96 场景风格一致。fast mode 覆盖。
2. **finalize 成功提示指向本地 merge**：CLI 子进程可驱动，但成功路径需真分支 + attach + base_sha，现有 `seed_bdd_project` fixture 不覆盖。apply 阶段评估：若新增 Given step 成本可控则升级 `@executable`，否则保留文档型 + 在 `tests/sdd_bdd_compat_tests.rs` 补一个 finalize 成功的集成测试（若 fixture 可行）。
3. **validate change 失败不诱导 delta**：CLI 子进程可驱动，需「格式不完整的 change」fixture。apply 阶段评估同上。

**默认假设**：若 fixture 成本过高，三个场景保持文档型（fast mode），合约层覆盖；P1 的 CLI 改动由 `tests/sdd_bdd_compat_tests.rs` 的既有 smoke + 单测兜底（finalize/validate 的核心逻辑已有测试覆盖，本次只是文案/分支调整）。

## 向后兼容

- 不改任何命令的退出码语义、flag 矩阵、文件命名。
- BDD-off 路径完全不变（next-steps 分支只影响 BDD-on）。
- 已安装的 `.agents/skills/**` 副本需用户跑 `llman sdd init --update` 才会拿到新叙事——这是既有的版本同步机制，不是回归。
- 下游项目（如 xylitol）升级 llman 后，重新 `init --update` 即可受益；无需改下游 config。

## 风险

- **低**：模板文案改动可能遗漏某处「Git/PR」表述——用 `grep -rn "Git/PR\|gh pr\|git push" templates/sdd/ docs/sdd/ AGENTS.md` 全量复核。
- **低**：finalize 提示的默认分支名读取在 bare repo / detached HEAD 下可能失败——复用 finalize 既有的 branch 读取逻辑（已有错误处理）。
- **低**：validate next-steps 签名调整可能漏改调用点——编译期捕获（Rust 类型系统）。

## P2 补充决策

### 5. change id 推导规则保持开放，不硬编码

**背景**：r99 要求「从描述内容直接生成合法且有意义的 change id」。初稿曾写死「动词前缀 add/update/remove/refactor/fix + 名词摘要，截断至 40 字符」。

**决策**：**不写死命名规约**。理由：
- `validate_sdd_id` 的合法底线很宽（非空、不含 `/`、非 `.`/`..`）——这是唯一的硬约束。
- 「有意义」是语义判断，不同项目有不同惯例。本项目 `llmanspec/AGENTS.md` 未显式规定 change 命名，现有 change 名风格混杂（`feat-sdd-status-compact`、`remove-future-planning-concept`、`bdd-on-local-promote`）。
- 若 CLI 写死「动词前缀 + 40 字符」，会与某些项目的既有惯例冲突（如本项目 `bdd-on-local-promote` 没有动词前缀）。
- 用户明确要求「遵循 `llmanspec/AGENTS.md`」——若该文件声明命名约定则遵循，否则按描述语义合理命名。

**实现**：`derive_change_id(desc: &str) -> Result<String>` 做最小清洗：
1. 读 `llmanspec/AGENTS.md`，若含明确的 change 命名规则段（如 `change 命名:` / `change-naming:`），按其执行（本版仅做文本提示，不做 AST 解析——agent 读后自行遵循）。
2. 否则：从描述提取关键词，转 kebab-case（小写、空格/标点转 `-`、合并连续 `-`、去首尾 `-`），过 `validate_sdd_id`。
3. 长度上限保留（如 60 字符，防止目录名过长），但这是卫生措施，不是命名规约。

**skill 层**：propose 模板的「轻量 draft 路径」节告诉 agent「从描述生成合法且有意义的 id，遵循项目 AGENTS.md 命名约定」——把命名判断交给 agent（它能读 AGENTS.md 并理解语义），而非用 CLI 的死规则。

### 6. `--from` 与 `<CHANGE>` 的互斥

`change new` 的 `<CHANGE>` 改为 `Option<String>`，加 `--from <description>`。clap 用 `group` 或手动校验：
- 两者皆无 → 报错「必须提供 `<CHANGE>` 或 `--from <description>`」
- 两者皆有 → 报错「`<CHANGE>` 与 `--from` 互斥」
- 仅 `<CHANGE>` → 既有行为不变
- 仅 `--from` → 推导 id，stdout 多打印一行 `derived id: <id>`

这保持了既有调用的向后兼容（只传 `<CHANGE>` 不受影响）。

### 7. 轻量 draft 路径不强制 Preflight

propose skill 的 Preflight（`validate --all --strict` + spec valid_scope 检查）对「快速记一个提案」是重负载。轻量 draft 路径**跳过**完整 Preflight：只检查 `llmanspec/` 存在（`change new` 本身已通过 `load_required_config` 守卫）。若 draft 后用户要求正式化（完整 propose），那时再跑 Preflight——避免在「只记一句话」的场景触发全量校验噪音。

### 8. `change new --from` 消息风格沿用 anyhow，不新增 locale key

**背景**：cli-experience r43 要求「运行时提示/状态/错误 MUST 优先用 `t!` 本地化键」。`change new --from` 新增了若干边界消息（互斥校验、必填校验、derived id 公告）。

**决策**：这些消息**沿用既有 `anyhow!` 字符串风格**，不新增 locale key。理由：
- 既有 `change new`（`change already exists` / `change proposal already exists`）及其它 change 子命令（attach/checkpoint/archive 的部分 anyhow bail）都用 anyhow 字符串，未走 `t!`。
- 只有面向用户的成功 stdout（如 `sdd.archive.archived`）走 `t!`；边界/错误消息既有混用。
- 本轮 finalize 的成功提示（P1-14）走 `t!`（与 archive 成功提示一致）；change new 的 derived id 公告是次要 stdout，与既有 change new 的 `println!(proposal_path)` 同风格（裸字符串）。
- 若日后统一 change 子命令的消息到 locale，应作为独立 change 处理（涉及 attach/checkpoint/archive 等多命令），不在本论范围。

这是对 r43「优先」的合理权衡——不新增半套 locale key 造成风格不一致。
