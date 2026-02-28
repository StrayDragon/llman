## Context

`agent-tools-usage-stats` 已实现完整的数据读取、聚合、JSON 输出与基础 TUI，但目前“呈现层”仍以最小可用为主：

- CLI `--format table` 由字符串拼接构成，列对齐/截断/可读性一般。
- TUI 的 Overview/Trend 以文本段落为主，Sessions 也以“拼接行”呈现，信息密度与视觉层次不足。
- Claude Code 的 sidechain 统计在数据层已就绪，但在 UI 上缺少更直观的对比表达（primary vs sidechain vs overall）。

本设计聚焦“输出美化”，要求不改变统计口径、不破坏 JSON 输出的机器可读性与稳定性，同时默认颜色策略为 `auto`（TTY 开启、非 TTY 关闭、遵循 `NO_COLOR`）。

## Goals / Non-Goals

**Goals:**
- 将 CLI table 升级为真实表格渲染（`comfy-table` / `tabled`），改善列对齐、截断、数字可读性。
- 增加颜色策略：默认 `auto`，并对 `NO_COLOR`/非 TTY 做正确降级；（可选）提供 `--color auto|always|never` 覆盖策略。
- TUI 改造为组件化布局：Overview 指标卡 + coverage gauge；Trend 图表化；Sessions 用 Table + 详情面板。
- 输出一致性：Claude 视图中能更清晰表达 primary/sidechain/overall（保持 known-only 口径）。
- 保持 JSON 输出无 ANSI、结构稳定（允许新增字段但必须向后兼容）。

**Non-Goals:**
- 不修改任何数据源读取逻辑与统计口径（只改渲染与交互层）。
- 不引入大型 UI 框架或跨工具统一大仪表盘。
- 不新增费用估算/货币换算。

## Decisions

### 1) CLI 表格库选择：优先 `comfy-table`，渲染接口可替换

选择 `comfy-table` 作为默认实现：
- 适合终端表格样式与颜色控制；
- 支持列宽、对齐、截断与风格（边框/无边框）；
- 更容易为不同 view（summary/trend/sessions/session）做一致的排版。

备选 `tabled`：
- 适合“结构体 → 表格”的快速渲染；
- 但在风格/颜色/复杂布局（多行 header、右侧 mini 指标）上可控性略弱。

实现上将把渲染逻辑抽象为 `render_table_*` 层，以便未来替换实现而不影响 core 结构。

### 2) 颜色策略：默认 `auto`，尊重 TTY 与 `NO_COLOR`

默认策略：
- `auto`：stdout 为 TTY 且未设置 `NO_COLOR` → 启用颜色；否则禁用颜色。
- JSON：永远无 ANSI（无论 `--color` 设置）。

可选 CLI 覆盖：
- `--color auto|always|never`：
  - `always`：即使非 TTY 也输出 ANSI（便于某些 pager/录屏场景）。
  - `never`：强制关闭 ANSI。

替代方案：
- 只依赖 `NO_COLOR`，不提供 `--color`。优点是 CLI 更简单；缺点是调试/用户控制力弱。

### 3) 数字与字段格式：人类可读但不破坏语义

- token 计数：千分位（例如 `12_345` 或 `12,345`），unknown 用 `-`（或 `—`）显式表达。
- Claude：在 summary/trend 中同时展示 overall 与 primary/sidechain（overall 仍是 totals；primary/sidechain 为对比列）。
- Sessions：增加 sidechain 标记列（Claude），unknown token 以灰色/暗色弱化，异常高 token 行高亮。

### 4) TUI：从“Paragraph 文本”升级到 “Table + Chart + Gauge”

布局建议：
- Overview：多列卡片（tokens / sessions / coverage / latest），coverage 使用 Gauge（已知会话覆盖率）。
- Trend：BarChart/Chart + 明细表格（bucket / overall / coverage），Claude 可切换视角（overall/primary/sidechain）。
- Sessions：Table 组件（end / tokens / id / sidechain / cwd / title）+ 右侧 detail panel（时间、duration、breakdown、cwd）。

风格建议：复用仓库现有 TUI（例如 `skills` picker 的高亮、底部 help 行样式），减少视觉割裂。

### 5) 测试策略：避免脆弱快照，覆盖关键行为

- 保持现有 JSON 集成测试不受影响。
- 新增/补充：
  - `--color auto` 在捕获输出（非 TTY）时不包含 ANSI；
  - `NO_COLOR=1` 强制无 ANSI；
  - （若引入 `--color always`）则应包含 ANSI 片段（宽松匹配 `\\x1b[`）。
- TUI 不做终端快照测试，仅保留纯状态机测试（如已有模式）。

## Risks / Trade-offs

- [表格输出变更导致用户脚本解析失败] → 强调 JSON 是脚本化接口；文档明确 table 不保证稳定；可考虑提供 `--format tsv`（非本次必须）。
- [ANSI 颜色在某些环境显示异常] → 默认 `auto` + `NO_COLOR` + `--color never` 兜底。
- [TUI 改造工作量偏大] → 拆分为小任务、逐步替换组件（先 Sessions Table，再 Overview/Trend）。

