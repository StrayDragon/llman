# 2026-04-08 — Sync Ignore (OpenCode / Cursor / Claude Code) Design

## Summary

提供 `llman tool sync-ignore`（alias `si`）来统一管理项目内各家工具的“额外 ignore 配置”，把它们解析为统一结构 `{ignore, include}`，再按用户选择同步回：

- OpenCode：`.ignore`
- Cursor：`.cursorignore`
- Claude Code：`.claude/settings.json`、`.claude/settings.local.json`（仅 `permissions.deny` 的 `Read(...)`）

默认行为强调**安全与开箱即用**：
- 默认 dry-run（只预览不落盘）
- `--yes` 才会创建/写入文件
- 默认必须在 git repo 内运行（找不到 `.git` 需 `--force`）
- 非交互也能自动创建缺失目标文件（满足“一键同步”）

## Scope / Non-scope

### In scope
- `.ignore` / `.cursorignore`：gitignore 风格（含 `!pattern`）
- `.claude/settings.json` / `.claude/settings.local.json`：JSON/JSONC（best-effort 保留注释）
- union 同步（ignore/include 分别 union + 去重 + 稳定输出）

### Out of scope（v1）
- `.gitignore` 转换/同步（各工具默认 respect）
- Claude Code user/global scope（仅项目内）
- 完整保留 gitignore 原始顺序语义（v1 使用稳定排序，`!pattern` 统一后置）
- include（`!pattern`）写入 Claude（deny-only，无法表达）

## Canonical Model

内部统一结构：

```text
IgnoreRules:
  ignore  : set<pattern>
  include : set<pattern>     # 来自 gitignore-like 的 !pattern
```

约定：
- 输出 `.ignore/.cursorignore`：先 ignore、后 include（`!` 前缀）
- 输出 Claude：只输出 ignore（转成 `Read(./pattern)`），include 跳过并告警

## File Mapping

### `.ignore` / `.cursorignore`
- `# ...` 注释行：跳过
- 空行：跳过
- `!dist/`：记录为 include(`dist/`)
- 其他行：记录为 ignore(line)

### `.claude/settings*.json`
- 读取 `permissions.deny` 数组
- 仅识别 `Read(...)`：
  - `Read(./secrets/**)` → ignore(`secrets/**`)
  - `Read(.env)` → ignore(`.env`)（写回时统一标准化为 `Read(./.env)`）
- 非 Read deny 项：
  - 解析时跳过并在 `--verbose` 提示
  - 写回时必须保留（不删除）

## CLI Design

### Command
- `llman tool sync-ignore`（alias: `si`）

### Options
- `--yes, -y`：应用写入（默认 dry-run）
- `--interactive`：启用 inquirer 交互（targets 多选 + 删除提示 + 预览确认）
- `--force`：跳过 git root 检查，把 cwd 当 root
- `--verbose, -v`：打印转换细节与所有 warning
- `--target, -t <target>`：可重复（或 `all`）
  - `opencode` → `.ignore`
  - `cursor` → `.cursorignore`
  - `claude-shared` → `.claude/settings.json`
  - `claude-local` → `.claude/settings.local.json`
  - `all` → 上述全部（但默认策略仍可对 local 做“仅存在才更新”）
- `--input, -i <path>`：可重复，额外输入文件（自动识别格式）

### Default behavior

1) **Root 决策**
- 默认向上找 `.git` 作为 repo root
- 找不到：报错，提示 `--force`

2) **Sources 自动发现**
- 存在才读：`.ignore`、`.cursorignore`、`.claude/settings.json`、`.claude/settings.local.json`
- 若用户提供 `--input`，则把它们追加为 sources

3) **Targets 默认输出**
- 非交互默认 targets：
  - 写入/创建：`.ignore`、`.cursorignore`、`.claude/settings.json`
  - `.claude/settings.local.json`：仅在已存在时更新（除非显式 `--target claude-local`）

4) **Dry-run / Apply**
- 默认 dry-run：打印预览并退出 0
- `--yes`：执行写入/创建；写入使用原子写入策略

## Preview Output (non-interactive)

目标：让用户在 dry-run 一眼看懂“会改哪些文件、改多少、有什么风险”。

建议输出结构：

1) Header
- repo root
- sources 列表（exists/missing）
- union 统计：ignore/include 数量

2) Summary table（简表）
- 每行一个 target：
  - `TARGET` / `ACTION` / `ignore` / `include` / `notes`

3) Details（按 target）
- `.ignore/.cursorignore`：
  - 展示将写入的文件内容（必要时限制行数，`--verbose` 展开）
- `.claude/settings*.json`：
  - 不展示整个 JSON
  - 仅展示 `permissions.deny` 将新增的 `Read(./...)` 列表
  - 若 fallback 到 pretty JSON 重写：在 notes 里显式提示“可能丢 JSONC 注释”

## Inquirer UX (interactive)

### Step 1 — MultiSelect targets
提示文案（示例）：
- Title: `Select outputs to sync`
- Items（标注 exists/missing）：
  - `.ignore (missing → will create)`
  - `.cursorignore (exists → will update)`
  - `.claude/settings.json (missing → will create)`
  - `.claude/settings.local.json (exists → will update)`

默认勾选建议：
- `.ignore`、`.cursorignore`、`.claude/settings.json`（即使 missing 也默认勾选，满足“开箱即用”）
- `.claude/settings.local.json`：仅在已存在时默认勾选

### Step 2 — Deselect deletion confirmation
当用户把“已存在文件”从勾选中移除时，逐个询问：
- `You deselected <file>. Delete it?`（默认 No）

### Step 3 — Preview
展示与 non-interactive 相同的 Preview（Summary + Details）。

### Step 4 — Confirm apply
最后询问：
- `Apply changes?`（Yes/No）
取消则无任何落盘。

## Claude Code JSONC Preservation (best-effort)

策略：
- 读取：`serde_json` 失败时使用 `llm_json` 修复/去注释后解析
- 写回优先：扫描原文本，定位 `permissions.deny` 的数组 `[...]` 区间，只替换该区间文本
- 定位失败 fallback：pretty JSON 全量重写，并明确提示“会丢注释/格式”

## Testing Policy (v1)

- 测试只覆盖核心逻辑：解析、union、渲染、Claude deny patch 合并与保留策略
- 不做 inquirer 交互集成测试
