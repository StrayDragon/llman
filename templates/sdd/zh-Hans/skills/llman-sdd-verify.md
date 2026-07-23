---
name: "llman-sdd-verify"
description: "验证已实施的 llman SDD 变更是否与 specs/design/tasks 一致。产出分级报告（CRITICAL / WARNING / SUGGESTION），对比代码与工件。在 apply 完成后运行；全绿则可归档。"
metadata:
  version: "{{ llman_version }}"
  llman_sdd:
    bdd_mode: "{{ bdd_mode }}"
    skill_set: "{{ skill_set }}"
---

# LLMAN SDD Verify

使用此 skill 验证实现是否与该 change 的 artifacts 一致。

## Pipeline 位置

```mermaid
flowchart LR
    apply["llman-sdd-apply<br/>实施"] --> verify
    verify["★ llman-sdd-verify ★<br/>验证（你现在在这里）"]
    verify --> archive["llman-sdd-archive<br/>归档"]
    archive --> commit["git commit<br/>完成闭环"]

    style verify fill:#fff3cd,stroke:#ffc107,stroke-width:3px
```

> 📍 你现在在验证阶段 → 通过后下一步 `llman-sdd-archive`（归档）；失败则回到 `llman-sdd-apply`（修复）

## 硬约束

- **必须先通过 apply 阶段全绿**：未完成实现的 change 跳过验证。
- **CRITICAL 必须修复**：标记为 CRITICAL 的问题归档前必须修复。
- **不要问「要不要继续」**：跑完整个验证流程，输出完整报告。

## 步骤
1. 确定 change id（不明确时让用户从 `llman sdd list --json` 选择）。
2. 检查阶段守卫（权威）：
   ```bash
   stage=$(llman sdd show <id> --json --type change | jq -r .stage)
   ```
   （若无 `jq`，可用任意工具从 JSON 中解析 `stage` 值。）
   - 若 `stage` 不为 `full`，变更尚未实现、无可验证内容 → 必须停止并给出守卫提示：
     - `draft`："变更 <id> 是 draft 提案（仅 proposal.md），尚无可验证的实现。请先用 llman-sdd-propose 生成完整工件，再用 llman-sdd-apply <id> 实现。"{% if bdd_enabled %} BDD-on 下，已有 proposal+design+tasks 仍是 `draft` 意味着变更**未 attach** —— 修复方式是 `llman sdd change attach <id>`（而非新增 `changes/<id>/specs/`）。{% endif %}
     - 其他非 full 阶段（`specified`/`designed`）："变更 <id> 处于 <stage> 阶段，尚未准备好被验证。请先用 llman-sdd-apply 实现。"
3. 先跑一个快速校验门禁：
   - `llman sdd validate <id> --strict --no-interactive`
   - **诊断结构问题（Gherkin 解析 / `@req` 链接 / 双写 / 全局 req_id 唯一性）时优先加 `--no-check`**（BDD-on 下跳过可能耗时的 `bdd.run_command`），结构门禁全绿后再跑完整 `--check`（full mode）。`FAIL <item_type>/<id>` 行会逐条列出失败项（在 Totals 行上方）。
4. 阅读：
{% if bdd_enabled %}
   - feature 分支上的 live specs：`llmanspec/specs/**`（`spec.toon` + `*.feature`）——BDD-on 下这是 SSOT
   - `proposal.md` 与 `design.md`（如存在）
   - `tasks.md`（理解实现范围）
   - change 内 `llmanspec/changes/<id>/specs/` 仅当残留文档存在时（优先读 live specs）
{% else %}
   - `llmanspec/changes/<id>/specs/` 下的 delta specs
   - `proposal.md` 与 `design.md`（如存在）
   - `tasks.md`（理解实现范围）
{% endif %}
5. **双轴审查（标准轴 + 合约轴分离，互不掩盖）**——对比 diff（`git diff <merge-base>...HEAD`，merge-base 取 attach 的 base_sha 或 `main`）分两轴：
   - **合约轴（Spec）**：实现是否满足 `spec.toon` 的 MUST/SHALL 与 `*.feature` 的 GWT。
     - 缺失/部分实现的行为、错误实现、以及 diff 中未被 spec 要求的超范围改动。
     - 给出最小修复建议，或建议更新 artifacts。
   - **标准轴（Standards）**：代码是否符合 `AGENTS.md` 的编码规范 + 常见代码坏味（code smell）清单。
     - **权威优先级**：`AGENTS.md` 文档规范 > 坏味清单（文档说了算）；工具已强制的项跳过。
     - 坏味标记为**判断性提示**（「可能是 Feature Envy」），不是硬性违规。
     - 坏味清单（每项「是什么 → 怎么修」）：Mysterious Name（名不达意→重命名）/ Duplicated Code（重复逻辑→抽取共享）/ Feature Envy（方法更爱用别人的数据→移过去）/ Data Clumps（同组字段到处走→打包成类型）/ Primitive Obsession（原始类型充当领域概念→给专门类型）/ Repeated Switches（同类 switch 反复出现→多态或共享 map）/ Shotgun Surgery（一处改动散落多处→聚到一模块）/ Divergent Change（一文件因多无关原因被改→拆分）/ Speculative Generality（为未发生的需求加抽象→删除）/ Message Chains（长链 a.b().c()→隐藏于一方法）/ Middle Man（只转发→删掉直连）/ Refused Bequest（子类拒绝大部继承→改组合）。
   - 两轴可并行（sub-agent）审查；报告 MUST 分离呈现，MUST NOT 合并或交叉重排（一轴通过不能掩盖另一轴失败）。
6. **BDD-on 验证（Git-native Partitioned SSOT）**——仅当 `config.yaml` 含 `bdd:` 段时：
   - 确认 change 已 attach，且当前在对应 feature 分支上。
   - `llman sdd validate --specs`：Gherkin + `@req`/双写门禁；默认跑 `bdd.run_command`（可用 `--no-check` 跳过）。
   - 可选只读审查：`llman sdd change diff <id>`（或 `--export-patch <path>`）。diff 仅作审查/导出——绝不当作 apply 步骤。
   - 归档：**优先** `llman sdd change finalize <id>`（可不要求干净树；随后一次 `git commit`）；需要严格 `checkpoint_sha` 时再走 `checkpoint` → `archive`。
   - 检查：可执行 GWT 只在 live `.feature`；`morphology.dualWriteCount` 应为 0；若已有活跃 `*.feature.delta.toon` 则先迁移（不要自创 solidify/找补步骤）。
{% if bdd_verify_prompt %}
   - 额外要求: {{ bdd_verify_prompt }}
{% endif %}
7. 输出简短报告：
   - **CRITICAL**（归档前必须修复）
   - **WARNING**（建议修复）
   - **SUGGESTION**（可选优化）
8. 若存在 CRITICAL，建议用 `llman-sdd-apply` 修复；若通过则建议归档：`llman sdd change finalize <id>`（推荐）或 fallback `checkpoint` + `archive`。

> 💡 验证通过 → 下一步 `llman-sdd-archive`（归档）；有 CRITICAL → 回到 `llman-sdd-apply`（修复）

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}
