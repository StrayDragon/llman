## Why

`llman sdd` 目前缺少 `llmanspec` 与 `openspec` 之间的标准化互转入口，团队在并行使用两套规范目录时需要手工搬运文件，容易遗漏 archive、元数据与规范约束。  
同时，这类目录级写操作如果被 agent/CLI 误触发会带来高风险，必须把“默认安全演练 + 人工确认”作为执行前提。

## What Changes

- 新增双向互转命令：
  - `llman sdd import --style openspec [path]`
  - `llman sdd export --style openspec [path]`
- `--style` 仅支持 `openspec`，且为必填；不提供 `migrate --from/--to` 兼容入口。
- 互转执行模型统一为“先演练后执行”：
  - 命令总是先生成并展示 dry-run 迁移计划
  - 仅在交互式终端中通过两次确认后才允许落盘
  - 非交互环境一律拒绝执行写入
- 迁移范围定义为完整迁移（`specs + active changes + archive changes`），并明确冲突与异常目录策略：
  - 目标冲突默认失败，不覆盖、不跳过
  - 非标准目录（如 `openspec/explorations/`）一并复制，并输出 warning 提示
- `export` 自动补齐 OpenSpec 侧关键元数据（`openspec/config.yaml`、active change 的 `.openspec.yaml`）；`import` 方向在缺失时补齐 llman spec frontmatter。
- 迁移写入完成后，在交互模式下提示用户是否删除旧迁移目录（默认不删除）；非交互场景不执行删除。

## Capabilities

### New Capabilities
- `sdd-openspec-interop`: 定义 `llman sdd import/export` 的接口、目录映射、元数据补齐与安全执行护栏。

### Modified Capabilities
- `sdd-workflow`: 扩展 `llman sdd` 命令面，纳入 `import`/`export` 并收敛旧的互转命名。

## Impact

- CLI 命令层：`src/sdd/command.rs` 需要新增子命令与参数校验。
- SDD 项目模块：新增互转执行器（计划构建、校验、确认、落盘）与路径安全校验逻辑。
- i18n：新增 `sdd.import.*` / `sdd.export.*` 文案键。
- 测试：补充集成测试覆盖双向迁移、非交互拒绝、冲突失败、非标准目录 warning+复制、旧目录删除确认（默认否）、元数据补齐、dry-run 输出一致性。
- 文档与规范：更新 `openspec/specs/sdd-workflow/spec.md`，并新增互转能力规范。
