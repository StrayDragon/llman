# Design — tool-agents-md-management

## 核心数据流

```
内置默认清单 [AGENTS.md, CLAUDE.md, ...]
        │ global config tools.agents-md 覆盖（非并集）
        ▼
  scan 扫描文件名清单 ──► 发现的路径（文件/目录）
        │ --upsert-project-configs
        ▼
  .llman/config.yaml tools.agents-md.files   ←─ SSOT（clean/revert 数据源）
        │
        ├── clean ──► 展开目录为 tracked 文件 ──► 删除 / git commit
        │             （默认分支门禁 + --force）
        │
        └── revert ─► 展开目录 ──► git checkout <default> -- <file> / git commit
```

## 关键设计决策

### 1. 清单来源：global 覆盖内置默认（非多级并集）
`resolve_scan_names()`：若 `global_config.tools.agents_md` 存在且 `files` 非空，用它；否则用 `default_agent_init_names()`。project config 的 `agents-md` 段**只作为 scan --upsert 的写入目标与 clean/revert 的数据源**，不参与「扫描哪些文件名」的定义。理由：合并语义简单可预测，避免「谁来扩展谁」的歧义。

### 2. 目录展开：文件级操作
`expand_to_tracked_files(root, paths)`：对清单中每项，若是目录，用 `git ls-files <dir>` 取该目录下所有 tracked 文件；若是文件，直接取（若 tracked）。遵守 .gitignore（`git ls-files` 天然遵守）。clean/revert 统一在文件级操作——git 不跟踪空目录，逐个文件删是 git 自然语义；revert 能精确 checkout 每个文件；dry-run 能精确列出。

### 3. 默认分支门禁（r122 安全核心）
复用 `src/sdd/change/git_native.rs` 的 `is_default_branch(root, &branch)`（已按 origin/HEAD → origin/main → origin/master → main → master 探测）。clean `--commit` 路径前置检查：默认分支且无 `--force` → `bail!` 拒绝。这些函数当前是 `pub`，可直接跨模块复用（同 crate 内）。

### 4. 提交语义
- clean `--commit`：单次 `git add <展开后的文件...>` + `git commit -m "chore(agents-md): clean stale agent init files"`。
- revert `--commit`：在非默认分支上直接 `git add` + `git commit`；在默认分支上自动 `git checkout -b agents-md/revert-<ts>` 后提交。

## 模块接口（depth 设计）

`src/tool/agents_md.rs` 顶层 3 个 `pub fn`：`run_scan` / `run_clean` / `run_revert`，各自接收 Args struct。内部抽出 `resolve_scan_names` / `expand_to_tracked_files` / `read_manifest` / `write_manifest` 4 个内部 helper，保持每个 fn 小而聚焦（对齐项目「small focused functions」规范）。

## 不做的事
- 不增加 `--from <ref>` / `--message` 等额外 flag（保持最小接口）。
- 不处理 worktree（单一工作区足够覆盖痛点）。
- 不改现有 tool 子命令行为。
