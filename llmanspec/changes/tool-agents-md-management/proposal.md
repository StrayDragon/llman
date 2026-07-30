---
depends_on: []
---

## Why

在团队合作项目中用个人 PR 开发时，agent init 文件（AGENTS.md / CLAUDE.md / .cursor/ 等）经常出现语义过期冲突：个人分支上被 agent 工具改写的 init 文件，在合并/rebase 时与主干版本冲突，污染 PR diff、阻塞合并。目前缺少把这些文件从工作区「临时移除 → 专心开发 → 从主干恢复」的受控流程。

## What Changes

1. **新增 `llman tool agents-md` 命令组**（3 个子命令），形成「扫描登记 → 按需清理 → 按需恢复」闭环：
   - `scan [--upsert-project-configs]`：递归扫描项目内 agent init 文件（支持文件名与目录），列出相对项目根的路径；`--upsert-project-configs` 时把发现的路径写入 `.llman/config.yaml` 的 `tools.agents-md.files` 段（不存在则创建）。
   - `clean [--commit] [--dry-run] [--force]`：读取 config 清单，将目录项展开为具体文件后逐个删除；`--commit` 时单次 `git add <files>` + 提交；**当前分支为默认分支（main/master）时拒绝执行，除非传 `--force`**。
   - `revert [--commit]`：从默认分支（探测顺序 origin/HEAD → origin/main → origin/master → main → master）`git checkout <default> -- <每个文件>` 恢复清单中文件到主干版本；`--commit` 时自动创建分支并提交。
2. **config schema 扩展**：`ToolsConfig` 新增可选 `agents-md: { files: [...] }` 段。扫描文件名清单来源 = global config 覆盖内置默认 `[AGENTS.md, CLAUDE.md, GEMINI.md, .cursorrules, .github/copilot-instructions.md]`。
3. **目录清单语义**：config 清单项既可是文件也可是目录（`.cursor/`）；`clean`/`revert` 运行时把目录展开为 git-tracked 文件逐个操作（遵守 .gitignore，用 `git ls-files` 锚定），不直接删整个目录。

## Capabilities

- 新建 `tool-agents-md-management`（r121: scan/clean/revert 合约；r122: 默认分支安全门禁；r123: 清单来源与目录展开语义）

## Impact

- **新增**：`src/tool/agents_md.rs`（核心实现）、`src/tool/command.rs`（3 个子命令 Args enum）、`src/cli.rs`（dispatch）、`src/tool/config.rs`（`AgentsMdConfig` schema）。
- **复用**：`src/sdd/change/git_native.rs` 的 `is_default_branch` / `resolve_default_branch_ref` / `run_git` / `current_branch`（git 安全门禁与操作）；`src/tool/sync_ignore.rs` 的 plan-preview / dry-run / comfy_table / atomic_write 模式。
- **i18n**：`locales/app.yml` 新增 `tool.agents_md.*` 键。
- **测试**：`tests/tool_agents_md_tests.rs`（集成测试，TempDir + 临时 git 仓库）。
- **不改**：现有 tool 子命令行为、sdd 流程、skills 逻辑。

## Open Questions（探索期已解决）

- [x] 扫描文件名清单来源 → global config 覆盖内置默认（非多级并集）。
- [x] 清单 SSOT 位置 → `.llman/config.yaml` 的 `tools.agents-md.files` 段。
- [x] commit message → 单次提交，固定前缀 `chore(agents-md): ...`。
- [x] revert 来源 ref → 默认分支（不增加 `--from`）。
- [x] clean 对目录 → 展开为 tracked 文件逐个删（非整目录删）。
- [x] scan --upsert 写入粒度 → 写实际发现的路径（文件或目录名）。
