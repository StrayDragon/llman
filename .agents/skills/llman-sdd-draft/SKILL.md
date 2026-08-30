---
name: "llman-sdd-draft"
description: "快速把一个 change 想法记成草案提案（仅 proposal.md，经 `change new --from`）。不强制 tasks/design/specs/attach。用于随手记 idea 或未来需求；准备好后用 propose 正式化。"
metadata:
  version: "0.0.71"
  llman_sdd:
    bdd_mode: "on"
    skill_set: "default"
---

# LLMAN SDD 草案（Draft）

把一个 change 想法记成**草案提案**（仅 `proposal.md` skeleton）。这是「先把 idea / 未来需求记下来」的轻量入口——不做 triage、不写 tasks、不编辑 live specs、不 attach。等想法准备好落实时，用 `llman-sdd-propose` 正式化。

## Pipeline 位置

> 📍 你现在在草案阶段 → 下一步：完善 `proposal.md`，然后运行 `llman-sdd-propose` 正式化
> 📎 本技能创建**草案** change（仅 proposal.md）。完整提案走 Git-native：tasks → Branch binding → Specs landing（见 propose 的生命周期图）
> 🗺️ Skill 导航 ≠ Git-native 生命周期；Branch binding / Specs landing 不是独立 skill

## 硬约束

- **MUST NOT 询问用户 change id**：由 `change new --from` 从描述推导并告知用户。
- **MUST NOT 创建 tasks/design/specs/attach**：本技能仅创建 `proposal.md` 草案壳。完整规划工件属于 `llman-sdd-propose`。
- **MUST NOT 做 triage 或判断变更规模**：那是 propose 的职责。若用户想开始实现，建议 `llman-sdd-propose`。
- **适用边界**：若描述明显涉及 MUST/SHALL 行为合约变更或多文件改动，建议用 `llman-sdd-propose` 而非停在草案——但仍先建草案壳以免想法丢失。
- **frontmatter 有固定 schema**：充实 `proposal.md` 时只接受 `llmanspec/AGENTS.md`「Change Proposal Frontmatter SSOT」中的合法字段（`depends_on`、`blocks`、`branch`、`base_sha`/`baseSha`、`checkpointed`、`checkpoint_sha`/`checkpointSha`、`skip_specs_landing`）。`status`/`title`/`priority`/`author` 等会被 `llman sdd validate` 报 ERROR 拒绝。生命周期阶段是推断量——用 `llman sdd show`/`list` 查看，绝不写进 frontmatter。正文 MUST NOT 复读 frontmatter 字段（不要 `## Status` 段）；正文 H1 用人类可读标题，不要复读 change id。

## 步骤

### 0) Preflight
- 读取 `llmanspec/config.yaml` 了解项目上下文、规则、locale。
- 必须存在 `llmanspec/`；若不存在，提示先运行 `llman sdd init`，然后 STOP。

### 1) 捕获描述
- 直接采用用户的描述（如「draft: 加一个导出 json 的命令」「记一下: sdd change 应该支持 worktree」）。
- **MUST NOT 询问 change id。** 由描述推导。

### 2) 创建草案壳
```bash
llman sdd change new --from "<用户描述>"
```
- CLI 会生成合法的 kebab-case id（清洗 + 校验），在 `llmanspec/changes/<生成的 id>/` 下创建 `proposal.md`（含 `## Why` / `## What Changes` TODO 段的 skeleton），并打印最终 id 与路径。
- 若生成的 id 与既有 change 冲突，CLI 以非零退出码失败；建议改写描述或用 `--force` 覆盖（对草案很罕见）。

### 3) 告知并交接
- **MUST 告知用户已生成的 id**（例如「已创建草案 change `<id>`，路径 `llmanspec/changes/<id>/proposal.md`」）。
- 建议下一步：
  - 现在或稍后完善 `proposal.md`（Why / What Changes / Capabilities / Impact）。
  - 准备好落实时，运行 `llman-sdd-propose` 正式化（triage + tasks → `change start`/`attach` → Specs landing）。

> 💡 草案已记 → 下一步：编辑 `proposal.md`，然后 `llman-sdd-propose` 正式化。

命令参考（由 CLI 命令树生成，始终与当前版本一致；细节用 `llman sdd <cmd> --help` 查看）：
- `llman sdd review` — 聚合审查：pending/manual 规则、未绑定场景、staleness、validate 全量扫描
- `llman sdd init` — 初始化 llmanspec（--update 刷新 skills/模板）
- `llman sdd list` — 列出变更或 specs
- `llman sdd show` — 查看 change 或 spec
- `llman sdd validate` — 校验 changes/specs（--strict 门禁；--no-check 跳过 BDD）
- `llman sdd archive` — `freeze` 把旧归档 change 目录冻结为单一冷备归档；`thaw` 从冷备归档恢复已归档 change
- `llman sdd change` — `new` 创建草案壳 proposal（--from 可从描述推导 id）；`attach` 把已有分支 + base SHA 绑定到 change；`start` Designed→Full：干净树门禁 + 建 sdd/ 分支 + 写绑定；`checkpoint` 为归档检查点干净且过校验的分支；`finalize` 单 commit 收尾：门禁 + ff-merge + 归档改名；`diff` 查看/导出 base...HEAD diff（--json 报告 commitCount）；`archive` 封存 change：ff-merge + 文档改名到 archive/
- `llman sdd spec` — `skeleton` 生成单轨 spec 骨架（直接通过 --strict）；`add-req` 向 spec 添加 requirement；`add-scenario` 为 requirement 添加验收场景；`next-req-id` 分配下一个空闲全局 req_id；`resolve-req` 解析 req_id 的归属 capability 与 statement
- `llman sdd graph` — 渲染 change 依赖图（mermaid）
- `llman sdd worktree` — `prune` 清理 change 已归档/缺失的 worktree
- `llman sdd context` — 查找与任务/路径相关的 specs（面向 agent）
- `llman sdd index` — `rebuild` 重建 spec 索引；`check` 检查索引新鲜度（不重建）
- `llman sdd config` — `skills` 交互式管理 extra_skills
- `llman sdd project` — `import` 从 OpenSpec markdown 导入 specs；`migrate` 迁移遗留 spec.toon 为单轨 .feature（仅 toon2features）；`dedupe-req-ids` 把冲突的 req_id 重映射为新的 rN 别名
本表由 `llman sdd init --update` 重新生成；MUST NOT 手写编辑。

## Ethics Governance
- `ethics.risk_level`：low——仅读写本仓库与 `llmanspec/`，无外发动作；正文另有声明时从其声明。
- `ethics.prohibited_actions`：违反正文「硬约束」的动作；未经用户明确要求的 push / PR / 外部上传。
- `ethics.required_evidence`：结论须有命令输出或文件路径佐证；门禁状态以 `llman sdd validate` 为准。
- `ethics.refusal_contract`：门禁 CRITICAL 未清零 → 拒绝进入下一阶段；自修复达上限 → 报告 blocker。
- `ethics.escalation_policy`：改动 SDD 合约/模板或执行不可逆动作前，暂停并请用户确认。
