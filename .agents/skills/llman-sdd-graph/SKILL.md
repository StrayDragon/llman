---
name: "llman-sdd-graph"
description: "以 mermaid 图可视化 llman SDD 变更间的依赖关系（depends_on/blocks）。辅助工具，任意阶段可用，不属于主实现 pipeline。"
metadata:
  version: "0.0.71"
  llman_sdd:
    bdd_mode: "on"
    skill_set: "default"
---

# LLMAN SDD 依赖图

使用此 skill 可视化变更之间的依赖关系。

## Pipeline 位置

> 📎 辅助工具，可在 pipeline 任意阶段使用。需要提案 → `llman-sdd-propose`；需要实施 → 仅当 `readyToImplement=true` 时用 `llman-sdd-apply`。

## 用法

**聚焦视图（seed 模式）：** 展示指定变更及其关系邻域。

```bash
llman sdd graph <change-id>              # 该变更 + 直接关系（depth 1）
llman sdd graph <change-id> --depth 3    # 递归 3 层
llman sdd graph <change-id> --depth 0    # 仅该变更自身
```

seed 模式沿 upstream（depends_on）、downstream（被谁依赖）、blocks 三个方向遍历，自动发现活跃和已归档变更。

**全局视图（scope 模式）：** 按范围展示所有变更。

```bash
llman sdd graph                          # 所有活跃变更（默认）
llman sdd graph --scope archived         # 所有已归档（已完成）变更
llman sdd graph --scope all              # 全部
```

## 输出

- 输出为 mermaid flowchart 到标准输出，可管道到文件或渲染器：
  ```
  llman sdd graph c50 > deps.mmd
  llman sdd graph c50 --depth 2 | mmdc -i - -o deps.png
  ```
- 已归档（已完成）变更以 "✓ done" 后缀和绿色高亮显示。
- 当图中存在互不相连的分组时，每组渲染为独立的 subgraph，标注 "Active"、"Done" 或 "Mixed"。

## 提案 frontmatter 格式

```yaml
---
depends_on:
  - other-change-id
blocks:
  - blocked-change-id
---

## Why
...
```

> 💡 这只是辅助工具 — 主流程：`llman-sdd-propose`（含 Branch binding + Specs landing）→ `llman-sdd-apply`（须 `readyToImplement`）→ `llman-sdd-verify` → `llman-sdd-archive`。

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
