## 背景
llman 需要一个内建的规范驱动开发工作流，但不依赖 Node 版 OpenSpec CLI。目标是镜像 OpenSpec 的核心行为（双目录模型、proposal/spec/design/tasks 工件、校验、归档），同时保持实现小而可控，并让 `openspec/` 与 `llmanspec/` 可共存。

## 目标 / 非目标
**目标：**
- 提供原生 `llman sdd` 命令组（init / update / list / show / validate / archive）。
- 在 `llmanspec/` 下提供与 OpenSpec 目录结构对齐的布局（`llmanspec/specs`、`llmanspec/changes`），并确保不修改 `openspec/`。
- 内置 spec-driven 模板与校验规则，行为与 OpenSpec schema 对齐。
- `llman sdd update` 写入并维护 `llmanspec/AGENTS.md` 中的 LLMANSPEC 受管块，用于在特定场景提醒 agent 使用 `llman sdd`。
- `list / show / validate` 提供 `--json` 输出并对齐 OpenSpec 的 JSON 结构。
- `archive` 提供 `--dry-run` 预检查模式。
- 不引入外部依赖、遥测或网络访问。

**非目标：**
- 交互式仪表盘或 TUI。
- 多 schema 或实验 OPSX 工作流（首版仅 spec-driven）。
- 编辑器特定 slash command 生成。
- `llman sdd` 不提供 `change/spec/view/completion/config` 等扩展子命令。
- 遥测或分析功能。

## 决策
- 以原生 Rust 模块实现（`src/sdd/**`），通过 `src/cli.rs` 集成，不调用外部 `openspec` 命令。
- 模板以资源文件形式内置（例如 `templates/sdd/spec-driven/`），由 init/update 写入 `llmanspec/templates/spec-driven/`。
- `llmanspec/AGENTS.md` 写入策略：使用受管块（`<!-- LLMANSPEC:START -->`/`<!-- LLMANSPEC:END -->`），`update` 仅替换该块并保留用户自定义内容；若文件缺失则生成完整模板。
- `list / show / validate` 默认文本输出，并提供与 OpenSpec 对齐的 `--json` 机器可读模式（含 `list --specs --json`）。
- `archive --dry-run` 仅输出将要修改/移动的文件与目标路径，不进行任何写入。

## 风险 / 权衡
- 与 OpenSpec 解析器存在细微差异的风险：用与 OpenSpec 对齐的模板与测试夹具缓解。
- 归档合并具有破坏性：默认强校验并提供 dry-run 预览。
- 仅支持 spec-driven 会限制高级用例：先确保核心闭环，后续再扩展。

## 迁移计划
- 纯新增能力，不影响现有 llman 命令。
- `llman sdd init` 为新项目提供 `llmanspec/` 基线结构（与 `openspec/` 共存）。
- README 增加 SDD 用法与迁移说明。

## 未决问题
- 暂无。

## 参考
- https://github.com/StrayDragon/OpenSpec/blob/main/README.md
- https://github.com/StrayDragon/OpenSpec/blob/main/openspec/AGENTS.md
- https://github.com/StrayDragon/OpenSpec/blob/main/schemas/spec-driven/schema.yaml
- https://github.com/StrayDragon/OpenSpec/blob/main/schemas/spec-driven/templates/proposal.md
- https://github.com/StrayDragon/OpenSpec/blob/main/schemas/spec-driven/templates/tasks.md
