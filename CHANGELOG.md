# Changelog

本文件记录用户可见的重要变更。格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循仓库 semver 惯例（0.0.x 连续递增）。

## [0.0.79] - 2026-09-17

### Added

- `llman-*` 外部子命令发现在 PATH 之外追加知名包管理器 bin 目录回退：
  `$BUN_INSTALL/bin`、`~/.bun/bin`、`$PNPM_HOME`、`~/.local/share/pnpm`、
  `~/.npm-global/bin`、`~/.local/bin`（PATH 命中保持优先；`bun link` 等开发
  安装即使不在 PATH 上也可被发现）。
- `llman sdd` 未安装外部 CLI 时，报错附带迁移指引文案（仅 `sdd` 触发，不污染
  其他未知命令的报错）。
- `migrations/v0.0.78-v0.0.79/`：升级 prompt（给用户/agent）+ 环境体检脚本
  `verify_sdd_external_migration.py`（纯检查：验证委托链路、残留清理、幂等不写文件）。

### Removed（破坏性）

- **移除内置 SDD 子系统（`llman sdd` / `llmanspec` 二进制 / `crates/llman-sdd` crate）**。
  功能已重写为独立项目 [llman-sdd v2](https://github.com/StrayDragon/llman-sdd)（TypeScript + Bun），
  以 npm 包分发：[`@llman-sdd/cli`](https://www.npmjs.com/package/@llman-sdd/cli)（命令 `llman-sdd` / `llmanspec`）+ `@llman-sdd/core`。
- `llman sdd` 不再是内置命令，也无弃用桩：`sdd` 未命中内置子命令时经 `llman-*`
  外部子命令发现**自动委托**给 PATH 上的 `llman-sdd`（argv 原样转发、stdio/cwd 继承、
  退出码透传、`-C/--config-dir` 以 `LLMAN_CONFIG_DIR` 注入）；未安装 llman-sdd 时
  报 `no llman-sdd executable found on PATH`。原 `llmanspec` 独立二进制随实现一并移除
  （命令名交还给 llman-sdd v2，避免 PATH 遮蔽）。
- 数据零迁移：`llmanspec/` 目录布局（`config.yaml` / `specs/*.feature` / `changes/`）不变，
  由 llman-sdd v2 直接读取；llman 自身不再生成/校验 `llmanspec-config.schema.json`
  （该 schema 工件与 `llman self schema` 对 llmanspec 的 apply/check 一并移除）。

### Removed（伴随清理）

- 随实现移除的测试与基础设施：`tests/bdd_steps.rs` BDD 步骤库与 `bdd` feature
  （rstest-bdd 三件套 dev-dependencies 一并移除）、`tests/it/sdd_*.rs` 三个 sdd 集成测试、
  `.agents/skills/llman-sdd-*` 生成技能、`scripts/check-sdd-templates.py` 门禁
  （`just check-all` 链条同步收窄）、`just test-bdd` / `just clean-bdd-targets`。
- workspace 收敛为 `llman` + `llman-core` 两个成员：移除 `crates/llman-sdd` 与
  `crates/gherkin-zh`（gherkin-zh 仍以独立 crate 存在于 crates.io，其 fork 致谢
  随源码迁出本仓库 NOTICE）；workspace 依赖 `minijinja` / `ureq` / `sevenz-rust2` /
  `gherkin-zh` 一并移除。
- `NOTICE` 中的第三方致谢段迁出：mattpocock/skills 致谢由 llman-sdd 仓库 NOTICE 承接。

### 迁移指引

```bash
npm install -g @llman-sdd/cli
llman-sdd --help   # 命令 llman-sdd / llmanspec，既有 llmanspec/ 项目直接可用
```

详见 <https://github.com/StrayDragon/llman-sdd>。
