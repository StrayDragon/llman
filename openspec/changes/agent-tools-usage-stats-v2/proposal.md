## Why

`agent-tools-usage-stats` v1 聚焦在“当前工作目录（cwd 精确匹配）+ 全历史实时扫描 + 无状态 TUI”。这能快速落地，但在真实使用中很快会遇到两类痛点：

- **范围痛点**：同一仓库/同一项目经常在不同子目录运行 agent CLI；仅 cwd 精确匹配会导致统计被切碎，用户需要频繁切换目录或手动调整查询范围。
- **性能痛点**：全历史实时扫描在 Codex breakdown / Claude 大量 jsonl / Cursor 大量 bubble KV 时会变慢；用户希望“第一次慢可以，后续秒开”。

因此需要一个 v2 来补齐“范围自动探测 + 可选索引缓存”的能力，让统计更贴近“项目级视角”，并显著改善使用体验。

> 状态：DRAFT（仅提案；后续再补 design/spec/tasks）

## What Changes

- 在 v1 的基础上扩展统计范围能力：
  - 新增 `--scope repo|all`（并保留 `cwd`），其中 `repo` 以 git 仓库根目录为边界，覆盖该 repo 内所有 cwd 记录
  - 增强路径规范化：可选 realpath/去符号链接，减少“同一目录多种表示”导致的误过滤
- 补齐 Cursor 覆盖面：
  - 为 Cursor 增加“传统 chat tabs”的 session 解析与 token 汇总（v1 仅覆盖 Composer）
- 引入**可选索引缓存**（默认开启或提供显式开关，待 spec 决定）：
  - 首次扫描构建索引；后续增量更新（按文件 mtime/大小或 sqlite updated_at）
  - 提供 `--no-cache` 强制实时扫描，用于排障/对账
  - 缓存存储在 llman 自己的目录（受 `LLMAN_CONFIG_DIR` 控制），不写入工具状态目录
- 提升 TUI 体验：
  - 在保持“默认无状态”的前提下，支持更快的刷新（来自索引）
  - 进度反馈与错误提示更细（例如 breakdown 文件解析失败的计数与跳过策略）

## Capabilities

### New Capabilities
<!-- 暂无：优先作为对现有 stats 能力的增强，而不是引入全新能力域 -->

### Modified Capabilities
- `agent-tools-usage-stats`: 扩展统计的 scope（repo/all）与路径规范化规则；新增/引入可选索引缓存机制与相关开关；提升大历史数据下的交互性能与可用性。

## Impact

- CLI/API：
  - `stats` 子命令新增/扩展参数（`--scope repo|all`、cache 开关、可能的路径规范化开关），需要文档化并提供示例
- 存储：
  - 新增 llman 自己的缓存文件（索引 DB/元数据），必须：
    - 不触碰真实用户配置（测试必须走 temp/`LLMAN_CONFIG_DIR`）
    - 不写入/修改 Codex/Claude/Cursor 的 session/state
- 风险：
  - 缓存一致性与失效策略不当可能导致统计偏差 → 需要明确的失效/重建策略与 `--no-cache`
  - repo scope 的路径判定在符号链接/多 worktree 情况下可能有歧义 → 需要明确规范化规则与测试覆盖
