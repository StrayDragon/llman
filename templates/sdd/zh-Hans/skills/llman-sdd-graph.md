---
name: "llman-sdd-graph"
description: "以 mermaid 图可视化 llman SDD 变更间的依赖关系（depends_on/blocks）。辅助工具，任意阶段可用，不属于主实现 pipeline。"
metadata:
  version: "{{ llman_version }}"
---

# LLMAN SDD 依赖图

使用此 skill 可视化变更之间的依赖关系。

## Pipeline 位置

```mermaid
flowchart LR
    pipeline["主 pipeline:<br/>propose → apply → verify → archive"]
    graph["📎 llman-sdd-graph<br/>依赖可视化（辅助工具）"]
    graph -.->|任意阶段可用| pipeline

    style graph fill:#e8f4e8,stroke:#28a745,stroke-width:2px
```

> 📎 辅助工具，可在 pipeline 任意阶段使用。需要落地执行时 → `llman-sdd-apply`（实施）或 `llman-sdd-propose`（提案）。

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

> 💡 这只是辅助工具 — 需要落地执行时回到主 pipeline：`llman-sdd-propose` → `llman-sdd-apply` → `llman-sdd-verify` → `llman-sdd-archive`。

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints-toon") }}

{{ unit("skills/structured-protocol") }}
