## Context
- 当前 skills 根目录固定为 `LLMAN_CONFIG_DIR/skills`，无法独立配置。
- 需要在不破坏默认行为的前提下增加可配置能力，并与现有 llman 配置文件共存。

## Goals / Non-Goals
- Goals:
  - 支持将 skills 根目录设为用户指定路径（如 `/home/.../llman.skills`）。
  - 明确优先级：CLI > ENV > llman 配置 > 默认值。
  - 保持现有 `config.toml`（sources/targets）格式不变，仅调整其所在根目录。
- Non-Goals:
  - 不改变 `LLMAN_CONFIG_DIR` 的解析逻辑。
  - 不引入新的 skills 配置文件格式或版本。

## Decisions
- Decision: 使用现有 llman 配置文件 `config.yaml` 增加 `skills.dir` 字段，仅用于 skills 根目录。
  - Reason: 复用现有配置入口，最小化新增文件与学习成本。
- Decision: `config.yaml` 读取遵循本地优先：先找当前目录 `.llman/config.yaml`，再回退 `LLMAN_CONFIG_DIR/config.yaml`。
  - Reason: 与现有 tool 配置逻辑保持一致，允许项目级覆盖。
- Decision: skills 根目录解析优先级固定为 `--skills-dir` > `LLMAN_SKILLS_DIR` > `config.yaml:skills.dir` > 默认 `LLMAN_CONFIG_DIR/skills`。
  - Reason: 显式覆盖优先，便于临时/自动化场景。
- Decision: `config.toml`（sources/targets）与 `registry.json`、`store/` 仍位于 skills 根目录下。
  - Reason: 保持现有技能管理的数据结构不变，降低迁移成本。

## Risks / Trade-offs
- 扩展 `config.yaml` 需要与现有工具配置兼容（缺省字段应能安全解析）。
- 新增 `--skills-dir` 和 `LLMAN_SKILLS_DIR` 需避免与其他子命令产生歧义。

## Migration Plan
- 无需数据迁移；默认路径不变。
- 若用户设置新的 skills 根目录，后续同步将在新目录创建 `store/registry/config.toml`。

## Open Questions
- None.
