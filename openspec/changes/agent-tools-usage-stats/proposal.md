## Why

目前在同一个项目里会同时使用 Codex CLI、Claude Code、Cursor（IDE/CLI）进行开发，但这些工具的 token 消耗、会话时长、峰值日期等信息分散在各自的本地状态文件里，用户难以：

- 快速回答“这个项目最近一周/一月大概消耗了多少 token？”
- 找到“哪一次会话/哪一天出现了异常高消耗？”
- 在不离开终端的情况下进行探索式下钻（趋势 → 会话列表 → 单会话细节）

因此需要在 llman 里提供一个**面向当前项目**、可在终端直观浏览的使用统计能力，帮助成本管理、性能/提示词优化与回溯定位。

## What Changes

- 为 3 个工具新增统计入口（互相独立的子命令，统一体验）：
  - `llman x codex stats ...`
  - `llman x claude-code stats ...`（含 `x cc` alias）
  - `llman x cursor stats ...`
- 每个 stats 子命令均提供：
  - 文本 CLI（表格/摘要 + 可选 JSON 输出，便于脚本与二次分析）
  - 终端 TUI（ratatui）用于交互式浏览趋势、会话列表与下钻详情
- 统计维度覆盖：
  - 时间聚合：按日/周/月
  - 会话维度：按 session/thread（可查看 token 与时长等元信息）
  - 过滤范围：第一版默认且仅支持当前工作目录（cwd 精确匹配）；repo/all 等更广范围后续再引入
- Token 口径策略：
  - **只展示已知 token**：不同工具/不同版本/不同事件可能缺少 token 字段；缺失时留空并在汇总中仅累计已知部分（不做估算）
- 数据源原则：
  - 只读取本机已有状态文件/数据库（不引入网络请求）
  - 不写入用户的 Codex/Claude/Cursor 状态目录（只读）
- 测试：
  - 使用最小化 fixture（sqlite/jsonl）覆盖解析、聚合、过滤、缺失字段容错等核心场景

## Capabilities

### New Capabilities
- `agent-tools-usage-stats`: 在 llman 中提供 Codex / Claude Code / Cursor 的本地历史会话统计能力（token、时间、趋势、下钻），并为三者统一展示范式（CLI + TUI），默认聚焦“当前项目”。

### Modified Capabilities
<!-- 无：该能力以新增 stats 子命令形式提供，不改变现有 export/account 等能力的既有 REQUIREMENTS -->

## Impact

- CLI 变更：在 `llman x codex|claude-code|cursor` 下新增 `stats` 子命令（新增而非替换）。
- 代码影响面：
  - 新增一套“统计查询 + 聚合 + 渲染（table/json/tui）”的共享实现，并在三个入口复用。
  - 需要读取用户主目录/应用状态目录中的 sqlite/jsonl 文件；需谨慎处理路径解析、权限错误与不同版本字段差异。
- 风险与约束：
  - 全历史扫描在大数据量下可能较慢；默认采取实时扫描但需提供 `--last/--since/--until/--limit` 等参数用于手动控时。
  - 隐私/机密：输出中不得打印 API key 等敏感环境变量；仅展示统计值与必要的会话元信息。
