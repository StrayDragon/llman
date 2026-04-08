## Context

**当前状态**
- OpenCode：项目根 `.ignore`（底层 grep/glob/list 等工具使用 ripgrep；`.ignore` 支持 gitignore 语法与 `!pattern` 显式包含）
- Cursor：项目根 `.cursorignore`（gitignore 风格，同样支持 `!pattern`）
- Claude Code：项目内 `.claude/settings.json` / `.claude/settings.local.json`，忽略相关规则位于 `permissions.deny`（`Read(...)`）
- 三种格式无法互操作，开发者需要手动维护多份配置
- 以上工具都会自动遵守 `.gitignore`，因此本工具仅处理“工具额外的 ignore 配置”

**约束条件**
- 不引入新的外部依赖（复用现有的 `ignore`, `inquire`, `serde_json`, `llm_json`, `anyhow`）
- 默认 dry-run，仅显式 `--yes` 才写入用户文件
- 必须在 git repo 内运行（找不到 `.git` 时需显式 `--force`）
- 写入 Claude Code settings 时 best-effort 保留 JSONC 注释与文件风格（失败时回退）
- 写入必须安全（原子写入、合理路径检查、错误消息清晰）

**利益相关者**
- 同时使用 OpenCode / Cursor / Claude Code 的开发者
- 需要在项目间保持忽略规则一致的团队

## Goals / Non-Goals

**Goals**
- 提供 `llman tool sync-ignore`：从任意一种输入解析为统一结构，并同步/转换到其它工具格式
- 支持自动发现项目内 ignore 配置并取 union（并集）后同步（“一键对齐”）
- 默认 dry-run，提供清晰预览；`--yes` 后才落盘
- 非交互模式也支持自动创建缺失的目标文件，开箱即用
- 提供交互式模式（MultiSelect 选择 targets + 反选删除提示 + 预览确认）
- 更新 `.claude/settings*.json` 时尽量保留 JSONC 注释（best-effort）

**Non-Goals**
- 不处理 `.gitignore`（各工具默认 respect）
- v1 不处理 Claude Code 的 user/global scope（仅项目内 `.claude/settings*.json`）
- 不保证完全保留 gitignore 的“顺序语义”（v1 输出为稳定排序；`!pattern` 统一后置）
- 不尝试把 `!pattern`（include）映射到 Claude Code（deny-only；将跳过并警告）
- 不覆盖/删除 Claude Code `permissions.deny` 中的非 `Read(...)` 规则（仅保留并做 union 增量合并）

## Decisions

### 1) Canonical Model：统一内部结构 `{ignore, include}`

**决策：**把所有输入解析为统一结构：

```rust
struct IgnoreRules {
  ignore: BTreeSet<String>,
  include: BTreeSet<String>,
}
```

**理由：**
- `.ignore` / `.cursorignore` 同为 gitignore 风格，天然可统一
- Claude Code 的 `permissions.deny` 可抽取 `Read(...)` 形成 `ignore` 集合
- union 同步与去重更直接
- BTreeSet 保证稳定排序，输出更可预测、diff 更小

### 2) Sources / Targets：自动发现 + union 同步

**决策：**
- **Sources（输入）**：默认自动发现并读取所有存在的文件：`.ignore`、`.cursorignore`、`.claude/settings.json`、`.claude/settings.local.json`
- **Targets（输出）**：默认（非交互）写入/创建：`.ignore`、`.cursorignore`、`.claude/settings.json`；`.claude/settings.local.json` 仅在已存在时更新
- union：把 sources 全部解析后取并集（ignore/include 分别 union）

**理由：**
- “自动识别 + union 同步”是核心用户价值：一条命令即可把多家工具对齐
- `.claude/settings.local.json` 属于本机配置，默认不主动创建更安全

### 3) `!pattern`（include）写入 Claude Code 的策略

**决策：**`include`（`!pattern`）无法映射到 Claude Code（deny-only），因此写入 Claude 时跳过并警告。

**理由：**
- Claude Code settings 的 `permissions.deny` 是拒绝列表，不存在“显式包含”语义
- 静默忽略会造成“看似同步成功但实际没生效”的安全误判

### 4) git repo 强约束 + `--force`

**决策：**默认必须在 git repo 内运行；从当前目录向上寻找最近 `.git` 作为 root。若找不到则报错，要求用户显式 `--force`（把当前目录当 root）。

**理由：**
- “项目根”语义清晰（`.ignore` 位于项目根）
- 避免在错误目录误创建 `.claude/` 等文件
- 与用户关于 OpenCode/rg 的心智一致（向上遍历到最近 Git 目录）

### 5) Claude Code settings：JSONC best-effort 保留注释

**决策：**
- 读取：优先 `serde_json`；失败则用 `llm_json` 修复/去注释后解析
- 写入：优先“局部替换 `permissions.deny` 数组区块”以保留 JSONC 注释与风格
- 回退：若无法可靠定位数组区块，则 fallback 为 pretty JSON 覆盖，并提示“可能丢注释”

**理由：**
- settings 文件可能包含 JSONC 注释；直接 pretty 重写会破坏用户手写内容
- 局部替换能最大化保留原文件结构，diff 更小

## CLI UX（含 inquirer 交互与预览）

### 非交互（默认）

- `llman tool sync-ignore`
  - 默认 dry-run：只预览不写入
  - 输出：
    - git root（或提示 `--force`）
    - 发现的 sources 列表（哪些存在、哪些缺失）
    - union 结果统计（ignore/include 数量）
    - 每个 target 的计划动作：create / update / unchanged
    - 警告列表（例如 include 无法写入 Claude）
  - 提示：使用 `--yes` 应用写入

- `llman tool sync-ignore --yes`
  - 应用写入
  - 自动创建缺失的 targets（以及必要的父目录，如 `.claude/`）

### Interactive Mode（inquire）

交互模式用于“选择输出目标/控制创建/可选删除”，流程如下：

1) **MultiSelect：选择输出文件（targets）**
   - `.ignore`（exists/missing）
   - `.cursorignore`（exists/missing）
   - `.claude/settings.json`（exists/missing）
   - `.claude/settings.local.json`（exists/missing）

2) **反选存在文件时提示是否删除（默认不删）**
   - 若用户将某个已存在文件从勾选中移除：询问“是否删除该文件？”默认不删除

3) **预览（Preview）**
   - 对每个 target 输出：
     - 动作：create / update / unchanged / (optional) delete
     - 规则数量：ignore/include（以及 Claude 的 Read 条目新增数）
     - 内容预览：
       - gitignore-like：直接展示将写入的行（稳定排序；include 行以 `!` 开头且统一后置）
       - Claude：展示将新增的 `Read(./...)` 列表（并提示哪些 include 被跳过）

4) **Confirm：确认执行**
   - 用户确认后才落盘；取消则不改任何文件

### 预览输出格式（建议）

- 摘要（简表）：
  - `TARGET` / `ACTION` / `ignore` / `include` / `notes`
- 细节（按 target）：
  - `.ignore` / `.cursorignore`：显示完整将写入内容（或限制行数 + `--verbose` 展开）
  - `.claude/settings*.json`：显示 `permissions.deny` 的新增项列表（不展示全文件）

## Risks / Trade-offs

### 风险 1: JSON 文件损坏或注释丢失

**风险：**局部替换失败导致 fallback pretty 重写，可能丢失 JSONC 注释/格式。

**缓解：**
- 默认 dry-run 提前预览
- 优先局部替换，仅在定位失败时回退并明确提示

### 风险 2: gitignore `!pattern` 的顺序语义

**风险：**gitignore 的否定模式与顺序有关；union + 排序会丢失原始顺序。

**缓解：**
- 输出顺序固定：先 ignore、后 include（`!`），让 include 尽量生效
- `--verbose` 下输出告警，提示“复杂顺序语义无法保证完全等价”

### 风险 3: “开箱即用”自动创建文件带来的惊讶

**风险：**`--yes` 可能创建 `.claude/`、`.ignore` 等新文件，影响仓库内容。

**缓解：**
- 默认 dry-run（用户需要显式 `--yes` 才会创建/写入）
- interactive 模式提供 MultiSelect 让用户控制创建范围

## Migration Plan

**部署步骤（预期）：**
1. 添加新模块 `src/tool/sync_ignore.rs`
2. 更新 `src/tool/command.rs` 注册 `sync-ignore` 子命令与参数
3.（可选）添加 `llman x cc sync-ignore` / `llman x cursor sync-ignore` 快捷转发
4. 更新 i18n 字符串
5. 添加核心逻辑测试（不覆盖 inquirer 交互）
6. 更新文档与示例

## Open Questions

1. **问：**是否需要支持 `.cursorindexingignore`（仅索引忽略）？
   **答：**v1 暂不支持，可后续按需求添加

2. **问：**是否需要默认创建 `.claude/settings.local.json`？
   **答：**v1 默认不创建，仅在已存在时更新；交互模式可选择创建
