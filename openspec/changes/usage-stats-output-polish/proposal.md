## Why

当前 `agent-tools-usage-stats` 的数据口径与功能已经完整，但输出层（CLI/TUI）仍偏“可用但朴素”：

- CLI `--format table` 目前更接近 TSV 文本，列对齐/截断/可读性一般，难以“一眼扫出异常高消耗与覆盖率”。
- Claude Code 的 primary/sidechain/overall 统计虽然已实现，但在 CLI/TUI 中缺少更直观的视觉编码（例如颜色、占比、对比列）。
- TUI 当前以段落文本为主（Overview/Trend），Sessions 也以拼接字符串呈现，信息密度与可视化程度不够。

因此需要一次面向“呈现层”的 polish：在不改变 JSON 稳定输出、不改变统计口径的前提下，让 stats 的默认体验更漂亮、更易读、更易下钻。

## What Changes

- CLI table 渲染升级：
  - 将 `--format table` 从“拼接文本”升级为“真正的表格渲染”（优先采用 `comfy-table` / `tabled`）。
  - 数字格式更友好（例如千分位、unknown 用 `-` 明确表示）。
  - 列对齐与截断更一致（cwd/title）。
- 颜色策略（默认 `auto`）：
  - 当 stdout 为 TTY 时默认启用颜色；非 TTY（管道/重定向）时默认关闭颜色。
  - 遵循 `NO_COLOR` 约定；可选提供 `--color auto|always|never` 以便强制覆盖。
  - JSON 输出保持无色、稳定、可脚本化（不引入 ANSI 码）。
- TUI 视觉与布局改进：
  - Overview：改为卡片/指标式展示（tokens、sessions、coverage、latest），覆盖率用 Gauge/Bar 直观呈现。
  - Trend：改为图表 + 明细（例如 BarChart/Chart + bucket 表格），Claude 可切换 overall/primary/sidechain 视角。
  - Sessions：使用 Table 组件呈现（列对齐、sidechain 标记、unknown 高亮），并提供右侧详情面板。
- 兼容性与安全：
  - 不改变现有数据读取与只读保证；不输出 secrets；不改变 JSON schema（除非新增字段且向后兼容）。

## Capabilities

### New Capabilities
<!-- 无：本次聚焦对既有 stats 能力的呈现层 polish -->

### Modified Capabilities
- `agent-tools-usage-stats`: 规范层面补充/强化“table 输出与颜色策略（auto）”与“TUI 展示要求”的用户可见行为。

## Impact

- 代码影响：
  - `src/usage_stats/render.rs`：表格渲染重构（引入表格库、颜色策略、数字/列格式化）。
  - `src/usage_stats/tui.rs`：布局与组件升级（Table/Chart/Gauge），并统一视觉风格。
  - 可能新增 `src/usage_stats/formatting.rs` / `render_table.rs` 等小模块以隔离渲染逻辑。
- 依赖影响：
  - 新增表格渲染依赖（`comfy-table` 或 `tabled`），以及必要的辅助依赖（如数值格式化/颜色控制）。
- 文档与测试：
  - 更新 `docs/usage-stats.md`（展示更漂亮的输出示例与颜色策略说明）。
  - 增加最小测试覆盖：颜色 auto 逻辑（TTY/NO_COLOR）与表格输出基础结构（不做脆弱的整段快照）。

