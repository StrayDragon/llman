---
name: "llman-sdd-quick"
description: "快速路径：处理不改行为合约的小改动——重构、修错字、性能优化。不涉及 MUST/SHALL 变更。如发现需要改合约，立即切换到 propose 完整路径。"
metadata:
  version: "0.0.71"
  llman_sdd:
    bdd_mode: "on"
    skill_set: "default"
---

# LLMAN SDD Quick Path

对于不涉及行为合约变更的小改动使用此路径。

## Pipeline 位置

> 📍 快速路径：不改行为合约，直接改代码 commit。如果发现需要改合约 → STOP，改走完整路径 `llman-sdd-propose`
> 🗺️ 完整路径含 Git-native Branch binding + Specs landing（不是把 Specs landing 当成独立 skill）

## 使用条件（所有条件必须满足）
- 不改变任何 spec 中 MUST/SHALL 定义的外部可观测行为
- 不涉及跨 capability 的修改
- 不涉及迁移/兼容性
- 不是 SDD 元规范变更

## 步骤
1. 用 `llman sdd context --task "..." --paths "..."` 确认无相关 spec 变更需要。
   - 如果 context 返回 `quality: "unavailable"`，运行 `llman sdd index rebuild`（默认 `pageindex`，无需模型）。
   - 可以用 `llman sdd list --specs --json` 查看 specs 元数据。
2. 直接修改代码。
3. 若要动 `llmanspec/specs/**`，STOP——除非已在绑定的非默认 change 分支上（迷你 change：`change start`/`attach` → 编辑 → commit）。禁止在默认分支 commit live specs，即使是 typo 或仅收紧 scope 也不行。优先把 live specs 维护路由到 `llman-sdd-propose`，或要求已有绑定分支。
4. git commit（message 写明 why）。
5. 无需 change 目录，无需 archive。

## 边界处理
- 如果在修改中发现需要改变行为合约 → STOP，改走 `llman-sdd-propose`（完整路径）。
- 如果涉及到多个文件且不确定 scope → 先用 `llman sdd context` 确认。

> 💡 快速路径完成 → git commit 即可。若需要走完整路径 → `llman-sdd-propose` → `llman-sdd-apply` → `llman-sdd-verify` → `llman-sdd-archive`

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
