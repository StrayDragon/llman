## Context

llman 目前已经具备对 Codex/Claude Code/Cursor 的若干“runner / account / export”类能力，但缺少对这些工具在**当前项目维度**的使用统计：

- Codex：本机状态中存在 thread/session 级别的 `tokens_used` 与 rollout jsonl（可包含 input/output/cache/reasoning 等细分），但没有在 llman 中可浏览的入口。
- Claude Code：本机 projects 的 session jsonl 内含 message usage（input/output/cache 等），并可能存在 sidechain/subagent；用户希望能够合并统计。
- Cursor：本机 `state.vscdb` 中可定位 bubble 的 `tokenCount` 与 `createdAt` 等字段，但需要将 KV 记录与会话/对话概念映射。

用户需求核心不是“精确计费”，而是：

- 以“历史趋势 + 会话下钻”为中心的可视化浏览（终端即可）
- 能覆盖三种工具、按日/周/月聚合、可筛选范围（默认 cwd）
- 对缺失 token 的数据不估算（保持可信），允许部分统计为空

约束与原则：

- **只读**本机状态文件；不写入/修改 Codex/Claude/Cursor 的状态目录
- 不依赖网络；避免引入新外部服务
- 需要可测试：解析与聚合逻辑必须能用最小 fixture 验证
- 可维护：三套入口独立，但共享内部统计模型与渲染逻辑

## Goals / Non-Goals

**Goals:**
- 为 `llman x codex|claude-code|cursor` 各新增 `stats` 子命令，提供：
  - CLI 表格摘要 + JSON 输出
  - 可交互 TUI（趋势/会话列表/详情）
- 支持统计视图：
  - `summary`（总览）
  - `trend`（按日/周/月聚合）
  - `sessions`（会话列表）
  - `session`（单会话下钻：CLI 模式用 `--id` 选择；TUI 通过列表下钻）
- 第一版默认且仅聚焦“当前工作目录（cwd 精确匹配）”（不做 repo/all 范围自动探测）
- Claude Code 默认包含 sidechain/subagent：既要分别计数，也要提供总计；并在 sessions 列表中作为独立记录可浏览/可筛选
- Cursor v1 仅统计 Composer sessions（传统 chat tabs 暂不覆盖，计划在 v2/后续补齐）
- Token 缺失时留空，不做估算；汇总仅累计已知部分
- 默认路径显示做截断（“仓库相对路径”优先，否则取最后两段）；`--verbose` 显示完整绝对路径
- TUI 提供可提交的“过滤表单”（范围/聚合/开关），避免用户必须记忆复杂 CLI 参数
- 聚合与显示遵循本机本地时区；按周聚合以周一为一周起点（ISO 周起点）
- `sessions` 视图默认按 end_ts 倒序展示，并提供默认 `--limit 200`（`--limit 0` 展示全部）

**Non-Goals:**
- 不实现统一的跨工具“大一统仪表盘入口”（本次按用户偏好保留每 tool 独立命令）
- 不做索引缓存/增量 DB（默认实时扫描；仅提供 `--last/--since/--until/--limit` 等参数以手动控时）
- 不进行费用估算/货币换算
- 不承诺覆盖 Windows 全部路径差异（保持当前“非主要目标平台”的策略）

## Decisions

### 1) 命令组织：每 tool 独立入口 + 共享内部实现

选择：
- `llman x codex stats ...`
- `llman x claude-code stats ...`（`x cc` 同样支持）
- `llman x cursor stats ...`

原因：
- 与现有 `x codex / x cc / x cursor` 的心智模型一致
- 每个工具的数据源差异较大（sqlite/jsonl/vscdb），独立入口更清晰
- 共享内部 `SessionRecord / TokenUsage / Query` 可避免重复实现

替代方案：
- 单入口 `llman stats --tool codex|cc|cursor`：减少命令数但增加分支复杂度，且与用户偏好不符
- 统一入口 + tab：后续可扩展，但本次先不引入

### 2) 数据口径：优先使用“本机权威字段”，缺失则留空

Codex：
- 优先读取 `threads.tokens_used` 作为会话总 token
- 可选 `--with-breakdown` 再解析 rollout jsonl 的 token usage 细分（input/output/cache/reasoning）

Claude Code：
- 以 projects session jsonl 的 `message.usage.*` 聚合为准
- `isSidechain` 默认计入（可参数关闭）

Cursor：
- 从 bubble JSON 的 `tokenCount.{inputTokens,outputTokens}` 提取（缺失则留空）
- v1 聚焦 Composer sessions（不依赖传统 chat tabs 的存储格式稳定性）

原因：
- “不估算”是用户明确偏好；保持可解释性与可信度
- 各工具/版本字段不稳定，估算会引入误差与争议

替代方案：
- 对缺失 token 做启发式估算：会提高覆盖率但降低可信度，且需要维护更多规则

### 3) 范围过滤：仅 cwd 精确匹配（v1）

默认：
- 仅统计 `cwd == 当前工作目录` 的记录（最直观的“当前项目”定义）

原因：
- cwd 精确匹配实现简单、可预期，避免“同一仓库多目录/多工作区”导致误归类

说明：
- repo/all 等更广范围的自动探测与过滤作为后续迭代能力单独引入（不在第一版范围内）

### 4) 展示：CLI + TUI 同构（overview/trend/sessions/session）

TUI 使用 ratatui（仓库已在用），采用同一套交互逻辑与布局范式，降低学习成本：
- overview：概览卡片 + sparkline
- trend：聚合图（按 day/week/month）
- sessions：会话列表 + 侧边详情预览
- session：单会话时间线（若数据源支持细粒度）
- 在 TUI 中提供“过滤表单”（例如快捷键打开 modal），可设置时间范围（since/until 或 all）、聚合粒度、Codex breakdown 等开关，并触发重新扫描/刷新

CLI 提供：
- table（默认）：适合快速查看与复制粘贴
- json：适合脚本化处理

原因：
- CLI 与 TUI 适配不同使用场景（脚本/交互）
- 同构可以复用渲染与查询逻辑，减少不一致

### 5) 性能策略：实时扫描为默认，但提供“缩小查询”参数

默认全历史实时扫描可能慢，但仍按用户偏好保留默认行为，同时提供：
- `--since` / `--until`（时间窗口，支持 RFC3339 与日期 `YYYY-MM-DD`）
- `--last <Nd>`（相对时间窗口，优先支持 `7d/30d/90d`，并作为 TUI 预设）
- `--limit`（会话列表限制）
- Codex breakdown 默认关闭（避免读取大量 rollout jsonl）

替代方案：
- 增量索引缓存：后续可作为性能增强能力，但本次不引入（避免引入新的持久化格式与迁移复杂度）

补充要求：
- 需要在长耗时扫描时提供进度反馈（CLI/TUI 至少其一有进度条/计数器），尤其是 Codex `--with-breakdown` 读取大量 rollout JSONL 时
- 全程只读：不得写入/修改用户的 session/state 文件（sqlite 连接需只读模式；jsonl 只读打开）

### 6) Rust 代码组织与可扩展性

为保证可扩展、可测试、可维护，采用“共享核心 + tool source 适配层”的组织方式：

- **共享核心模块**：新增 `src/usage_stats/`，仅负责“模型 + 查询 + 聚合 + 渲染 + TUI 状态”，不直接耦合具体工具的存储细节。
  - `model`: `TokenUsage` / `SessionRecord` / `SessionId` / coverage 字段
  - `query`: CLI 参数到 `StatsQuery` 的转换（含 `--since/--until/--last` 解析）
  - `aggregate`: bucketing（本地时区 + 周一周起点）、summary/trend/sessions/session 视图构建
  - `render`: table/json 输出（JSON 为稳定、可解析结构）
  - `tui`: ratatui UI（tabs + filter form + progress）
- **tool source 适配层**：每个工具实现一个 source（建议放在 `src/x/<tool>/stats.rs` 或同级模块），负责把本机存储解析为 `Vec<SessionRecord>`（或迭代器），并支持“路径 override”以便测试与排障。
  - Codex：只读打开 state sqlite（`mode=ro`）+ 可选解析 rollout JSONL（流式逐行）
  - Claude：扫描 projects 目录 JSONL（流式逐行），聚合到 sessionId
  - Cursor：复用现有 CursorDatabase（只读连接）并扩展 bubble tokenCount/createdAt 抽取逻辑（避免不必要加载大字段）
- **边界清晰**：
  - I/O（读文件/读 sqlite）只存在于 source 层；核心层只处理纯数据结构
  - 所有“缺字段/坏数据”在 source 层宽容处理为 unknown，并保证核心层不会 panic
- **测试策略**：
  - 核心聚合/时间解析：单元测试靠近模块（无文件依赖）
  - source：使用最小 fixture（sqlite/jsonl/vscdb）单测覆盖关键分支（createdAt 两种格式、token 缺失、sidechain 等）
  - CLI：`--format json` 集成测试使用 override 路径 + `TestProcess`（避免读取真实 home）

该结构允许后续：
- 新增更多工具（仅需实现 source + wiring）
- v2 引入 repo scope/索引缓存（核心模型与视图构建基本不变，只扩展 query 与数据来源）

## Risks / Trade-offs

- [大历史扫描慢] → 提供 `--since/--limit`；默认不做 breakdown；UI 展示“正在扫描”与进度提示（若可得）
- [字段/格式随工具版本变化] → 解析层做“宽容读取 + 关键字段缺失不报错”；为各工具准备最小 fixture 覆盖
- [隐私泄露风险] → 输出中禁止打印 env/headers；仅展示 token/time/title/cwd 等；必要时对路径做截断显示
- [Cursor bubbleId 映射复杂] → 以“尽可能提取 tokenCount”为目标，无法映射时仍可做趋势（基于已解析的 bubbles）

## UX Preview（Draft, moved from docs/usage-stats-preview.md）

更新时间：2026-02-28

本节面向“用户侧操作”，描述 v1 `agent-tools-usage-stats` 的 CLI/TUI 体验预览与默认口径。

> v1 范围限制：只统计“记录的 cwd == 当前工作目录”的数据（不支持 repo/all）。更广范围见 v2 change：`openspec/changes/agent-tools-usage-stats-v2/`。
>
> Cursor v1 范围限制：仅统计 Composer sessions（传统 chat tabs 暂不覆盖，计划在 v2/后续补齐）。

### 1) 用户能做什么（典型工作流）

#### 1.1 快速总览（默认 summary）

- Codex：`llman x codex stats`
- Claude Code：`llman x claude-code stats`（或 `llman x cc stats`）
- Cursor：`llman x cursor stats`

输出（table）包含：
- known-only token 总计（仅累计已知 token）
- 覆盖率：`known_token_sessions / total_sessions`
- 最近活动时间、会话数等基础指标
- Claude Code 额外显示：primary / sidechain / overall 三段 totals（均为 known-only）

#### 1.2 看趋势（trend：按日/周/月）

示例：
- 最近 30 天按周聚合：`llman x cc stats --view trend --last 30d --group-by week`
- 指定日期范围：`llman x codex stats --view trend --since 2026-02-01 --until 2026-03-01`

说明：
- bucketing 使用**本机本地时区**
- week 以**周一**为一周起点

#### 1.3 找到高消耗会话（sessions：列表）

示例：
- 最近 7 天最新 20 条：`llman x cursor stats --view sessions --last 7d --limit 20`

列表默认：
- 按 end_ts 倒序（最新在前）
- `--limit` 默认 200；`--limit 0` 表示不限制
- 路径默认缩短显示；`--verbose` 才显示完整绝对路径

#### 1.4 下钻单会话（session：详情）

非 TUI 模式下，必须通过 `--id` 选择会话：
- `llman x codex stats --view session --id <THREAD_ID>`
- `llman x cc stats --view session --id <SESSION_ID>`
- `llman x cursor stats --view session --id <CURSOR_SESSION_ID>`

会话 id 规范：
- Codex：`threads.id`
- Claude Code：`sessionId`
- Cursor（v1）：`composer:<id>`

Codex 若想要 breakdown（input/output/cache/reasoning）：
- `llman x codex stats --view session --id <THREAD_ID> --with-breakdown`

### 2) CLI 设计预览（参数与默认值）

#### 2.1 共享参数

- `--view summary|trend|sessions|session`（默认 `summary`）
- `--group-by day|week|month`（默认 `day`；仅对 `trend` 生效）
- 时间范围（互斥规则）：
  - `--last <Nd>`（例如 `7d` / `30d` / `90d`）
  - 或 `--since <TIME>` / `--until <TIME>`（支持 RFC3339 与 `YYYY-MM-DD`）
- 输出：
  - `--format table|json`（默认 `table`）
  - `--tui`（进入 TUI；TUI 优先于 table/json）
- 列表/详情：
  - `--limit <N>`（仅对 `--view sessions` 生效；默认 200；`0` 不限制）
  - `--id <ID>`（`--view session` 且非 `--tui` 时必填）
- 路径显示：
  - 默认缩短（repo-relative 优先，否则 last-2-segments）
  - `--verbose` 输出完整绝对路径

#### 2.2 工具专属参数（调试/测试友好）

Codex：
- `--state-db <PATH>`：使用指定 `state_*.sqlite`，避免读取真实 `~/.codex`
- `--with-breakdown`：解析 rollout JSONL 获取 token 细分（更慢；全程只读）

Claude Code：
- `--projects-dir <PATH>`：扫描指定 projects 目录
- `--no-sidechain`：排除 sidechain/subagent

Cursor：
- `--db-path <PATH>`：指定 workspace `state.vscdb`
- `--global-db-path <PATH>`：指定 global `state.vscdb`（用于 bubble KV）

### 3) JSON 输出预览（供脚本/二次分析）

所有 `--format json` 输出：
- 必须是合法 JSON
- 必须包含 query 元信息（view/group-by/time-range/flags）
- totals 与 coverage 字段必须明确“known-only”口径

示例（summary，结构示意，字段名以实现为准）：

```json
{
  "tool": "codex",
  "view": "summary",
  "range": { "mode": "last", "value": "30d" },
  "totals": { "tokens_total_known": 123456 },
  "coverage": { "total_sessions": 42, "known_token_sessions": 40 }
}
```

### 4) TUI 设计预览（布局/交互/进度）

#### 4.1 布局（四个 tab）

- Overview：headline totals + coverage + sparklines（tokens/session count）
- Trend：按 bucket 的图表（day/week/month）
- Sessions：列表 + 侧边预览（标题/时间/token breakdown）
- Session Detail：单会话时间线（尽可能细分；不足则展示 totals-only）

#### 4.2 过滤表单（Filter Form）

通过快捷键打开 modal，修改后提交会触发重新扫描：
- 时间预设：All / 7d / 30d / 90d
- group-by：day / week / month
- 开关：
  - Codex：breakdown on/off
  - Claude：include sidechain on/off

TUI 第一版无状态：不把选择写入磁盘（每次运行从默认值开始）。

#### 4.3 进度条/计数器

当扫描较慢（尤其 Codex breakdown 解析大量 rollout JSONL）时：
- TUI 必须显示可见的进度反馈（例如“已处理 N/M 个文件/会话”）
- 对解析失败的文件提供计数与“已跳过”提示（不崩溃）

### 5) 安全与只读保证（用户侧可预期行为）

- 全程只读：
  - SQLite 数据库只读连接（`mode=ro` 或等价机制）
  - JSONL 只读打开
  - 不写入/修改 `~/.codex`、`~/.claude`、Cursor 状态目录
- 输出不打印 secrets（env/auth token 等），仅展示统计与必要元信息

## Migration Plan

- 该变更为新增命令，不替换既有行为；无数据迁移。
- 发布时在 changelog/README 中注明新增 `stats` 子命令与默认范围（cwd 精确匹配）与 token 口径（known-only）。
- 若未来引入缓存索引，需新增单独 change 并提供清晰的 cache 清理/禁用开关。

## Open Questions

- （v1 无阻塞性未决问题；后续增强项可在新 change 中提出）
