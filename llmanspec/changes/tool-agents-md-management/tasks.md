# Tasks — tool-agents-md-management

## Slice 1: config schema 基础（r121 数据源）

- [ ] T1 `src/tool/config.rs` 新增 `AgentsMdConfig { files: Vec<String> }`，挂到 `ToolsConfig.agents_md`（serde rename `agents-md`）。提供 `default_agent_init_names()` 返回内置默认清单。
- [ ] T2 单测：`ToolsConfig` 反序列化含 `agents-md` 段的 yaml；缺省时 `agents_md == None`；schema 校验通过。

## Slice 2: scan 命令（r121）

- [ ] T3 `src/tool/command.rs` 新增 `AgentsMd` 子命令 enum + `AgentsMdScanArgs`（含 `--upsert-project-configs`、`--config`、`--verbose`）。`src/cli.rs` dispatch 接线。
- [ ] T4 `src/tool/agents_md.rs`：`scan` 实现——解析清单来源（global config 覆盖默认），递归扫描项目内匹配文件/目录，输出相对项目根路径。
- [ ] T5 `scan --upsert-project-configs`：写入 `.llman/config.yaml` 的 `tools.agents-md.files`（文件不存在则创建，用 atomic_write）。
- [ ] T6 集成测试 `tests/tool_agents_md_tests.rs`：scan 列出文件；scan upsert 创建 config；global config 覆盖默认清单。

## Slice 3: clean 命令 + 默认分支门禁（r122）[blocked-by T4]

- [ ] T7 `AgentsMdCleanArgs`（`--commit`、`--dry-run`、`--force`、`--yes`、`--config`）。dispatch 接线。
- [ ] T8 `clean` 实现：读 config 清单 → 目录展开为 git-tracked 文件（`git ls-files`，遵守 .gitignore）→ plan preview（comfy_table）→ dry-run 或删除。
- [ ] T9 默认分支门禁：`--commit` 时复用 `git_native::is_default_branch` / `current_branch`，默认分支上拒绝（除非 `--force`）。`--commit` 单次 `git add <files>` + 提交 `chore(agents-md): clean stale agent init files`。
- [ ] T10 集成测试：clean dry-run 不删；clean --yes 删除文件；clean 目录展开；clean --commit 在 main 拒绝；clean --commit --force 执行。

## Slice 4: revert 命令（r123）[blocked-by T8]

- [ ] T11 `AgentsMdRevertArgs`（`--commit`、`--yes`、`--config`）。dispatch 接线。
- [ ] T12 `revert` 实现：读清单 → 展开目录 → `resolve_default_branch_ref` → 逐个 `git checkout <default> -- <file>`。`--commit` 自动建分支（`agents-md/revert-<timestamp>`）并提交。
- [ ] T13 集成测试：revert --yes 恢复文件到主干版本；revert --commit 建分支提交。

## Slice 5: i18n + 收尾

- [ ] T14 `locales/app.yml` 新增 `tool.agents_md.*` 全部键（start/error/preview/dry_run_hint/commit/revert 等）。
- [ ] T15 更新 `artifacts/schema/configs/en/llman-project-config.schema.json` 与 global schema（重新生成）。
- [ ] T16 `just check` 全绿（fmt + clippy + test）。
