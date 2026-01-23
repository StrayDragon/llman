## 1. CLI 脚手架与模板
- [x] 1.1 在 `src/cli.rs` 增加 `SddArgs` 与 `SddCommands`，新增 `src/sdd/mod.rs` 作为命令入口。
- [x] 1.2 添加 spec-driven 模板（proposal/spec/design/tasks），存放为资源文件或嵌入式字符串，目标路径为 `llmanspec/templates/spec-driven/`。
- [x] 1.3 实现 `llman sdd init [path]`：创建 `llmanspec/` 结构与模板并写入 `llmanspec/AGENTS.md`（含 `<!-- LLMANSPEC:START -->`/`<!-- LLMANSPEC:END -->` 受管提示块），不修改 `openspec/`。

## 2. 核心 SDD 命令
- [x] 2.1 实现 `llman sdd update [path]`：刷新指令与模板，仅更新 LLMANSPEC 受管块且不触碰 `llmanspec/specs/**` 与 `llmanspec/changes/**`，不修改 `openspec/`。
- [x] 2.2 实现 `llman sdd list`（`--changes`/`--specs`/`--sort`）并提供 `--json` 输出（changes 为 `{changes:[...]}`，specs 为数组）。
- [x] 2.3 实现 `llman sdd show <id> [--type change|spec]`：自动识别类型，原样输出 markdown，并提供 OpenSpec 对齐的 `--json`，支持 `--deltas-only`/`--requirements-only` 与 `--requirements`/`--no-scenarios`/`-r`。
- [x] 2.4 实现 `llman sdd validate`：支持 `--all/--changes/--specs/--type`、`--strict --no-interactive --json`，JSON 采用 OpenSpec 顶层结构（`items`/`summary`/`version`）。
- [x] 2.5 限定 `llman sdd` 子命令范围仅含 `init/update/list/show/validate/archive`，帮助输出不出现 `change/spec/view/completion/config`。
- [x] 2.6 对齐 `show/validate` 的交互选择与非交互提示语（提示文案与 OpenSpec 一致，命令名替换为 `llman sdd`）。
- [x] 2.7 为 `llmanspec/specs/<id>/spec.md` 解析 YAML frontmatter 校验元数据（`llman_spec_valid_scope/commands/evidence`），缺失或为空视为校验失败。
- [x] 2.8 在 `llman sdd validate` 中实现 staleness 校验（merge-base、scope diff、spec 更新检测、dirty 状态、子模块路径命中、`--strict` 升级）。
- [x] 2.9 扩展 validate 输出：JSON 新增 `staleness` 字段，文本模式在每项输出中提示 staleness 状态。

## 3. 归档流程
- [x] 3.1 实现 ADDED/MODIFIED/REMOVED/RENAMED delta 解析（`llmanspec/changes/<id>/specs/**/spec.md`）。
- [x] 3.2 实现 `llman sdd archive <id>`：合并 delta 到 `llmanspec/specs`，再移动到 `llmanspec/changes/archive/YYYY-MM-DD-<id>`，缺失 requirement 时报错中止。
- [x] 3.3 支持 `--skip-specs`（仅归档目录，不更新 specs）。
- [x] 3.4 支持 `--dry-run` 预览（输出将变更的文件列表与目标路径）。

## 4. 消息与文档
- [x] 4.1 在 `locales/app.yml` 增加 SDD 相关提示与错误文案，并通过 `t!` 输出。
- [x] 4.2 更新 `README.md`，补充 `llman sdd` 用法与示例。
- [x] 4.3 补充 spec frontmatter 校验元数据示例（README 或模板/说明文件）。

## 5. 测试与验证
- [x] 5.1 为校验与归档合并逻辑添加单元测试（含 dry-run 与 json 输出场景）。
- [x] 5.2 添加 init/list/show/validate/archive 的集成测试（临时目录）。
- [x] 5.3 验证：`just fmt`、`just lint`、`just test`，以及手动 smoke（如 `llman sdd init`、`llman sdd list --specs`、`llman sdd validate <id> --strict --no-interactive`）。
- [x] 5.4 添加 frontmatter/staleness 校验测试（scope 命中、base fallback、dirty 状态、strict 升级）。
