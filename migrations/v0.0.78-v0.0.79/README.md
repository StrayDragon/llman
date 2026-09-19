# 升级到外部 sdd 委托（v0.0.78 → v0.0.79）

## 升级 prompt（给用户/agent）

你的 llman 需要从 v0.0.78 升级到 v0.0.79。v0.0.79 移除了内置 sdd 子系统，
`llman sdd <cmd>` 不再是内置命令，而是通过外部子命令发现委托给可执行文件
`llman-sdd`（由独立的 llman-sdd v2 项目发行）。执行以下步骤：

1. **安装外部 llman-sdd CLI**（三选一）：
   ```bash
   bun i -g @llman-sdd/cli   # 或 pnpm i -g / npm i -g
   ```
2. **清理旧编译产物残留**（若存在，同名二进制会抢占 PATH、拦截委托）：
   ```bash
   rm -f ~/.cargo/bin/llman-sdd ~/.cargo/bin/llmanspec
   ```
3. **环境体检**（不写任何文件，无 `--apply` 步骤——本版本没有仓库内数据变换）：
   ```bash
   python3 migrations/v0.0.78-v0.0.79/verify_sdd_external_migration.py
   ```
4. **验证委托链路**（在任一含 `llmanspec/` 的项目下）：
   ```bash
   llman sdd --version
   llman sdd list --specs --json
   ```
5. **人工处理项**（无法自动处理，MUST 人工确认）：
   - `llmanspec/` 数据目录现归外部 llman-sdd v2 所有：校验/审查请改用
     `llman-sdd validate` / `llman-sdd review`（旧别名 `llmanspec <cmd>` 已废弃，
     将随下一个发布移除；`llmanspec/` 目录名不受影响）。
   - 依赖 llman 内置 sdd 实现的脚本或 CI，需改为直接调用 `llman-sdd`。
   - 通过 `bun link` 链接的开发版 `llman-sdd` 即使 `~/.bun/bin` 不在 PATH 上
     也能被自动发现（v0.0.79 新增包管理器 bin 目录回退）。

## 本版本破坏性变更（v0.0.79）

- **内置 sdd 子系统整体移除**：`crates/llman-sdd`、`crates/gherkin-zh`、
  sdd skills 模板、`llmanspec-config` schema 生成均不再随 llman 分发。
- **外部子命令委托契约**：`llman <name>`（无内置匹配时）解析 `llman-<name>`
  ——argv 原样转发、env 继承（`-C/--config-dir` 规范化为 `LLMAN_CONFIG_DIR`
  注入）、stdio/cwd 继承、退出码透传（Unix 信号死亡映射 `128+signum`）。
- **插件发现回退目录**：除 PATH 外追加 `$BUN_INSTALL/bin`、`~/.bun/bin`、
  `$PNPM_HOME`、`~/.local/share/pnpm`、`~/.npm-global/bin`、`~/.local/bin`
  （PATH 命中保持优先）。
- `llman sdd` 未发现外部 CLI 时，报错附带迁移指引文案（仅 `sdd` 触发）。

## 脚本行为（verify_sdd_external_migration.py）

- **纯体检**：不写任何文件（天然 dry-run）、幂等（全绿时重复执行 no-op）。
- 检查项：`llman` 在 PATH 上；`llman-sdd` 可发现（PATH 或 `~/.bun/bin` 回退）；
  `llman sdd --version` 端到端委托成功；`~/.cargo/bin` 无旧 `llman-sdd` /
  `llmanspec` 残留。
- 无法自动处理的项一律打印人工处理清单，不猜测、不改文件。
