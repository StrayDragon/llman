## Context
- llman 使用 `~/.config/llman/config.yaml` 与 `.llman/config.yaml` 承载工具与 skills 的部分配置，但缺少 schema 与补全提示。
- 目前没有统一的 schema 生成与校验链路，且默认配置文件不一定存在。
- 需要在不频繁改写用户配置的前提下改善 YAML LSP 体验。

## Goals / Non-Goals
- Goals:
  - 生成并维护 JSON schema，支持 YAML LSP 补全与校验。
  - 提供全局配置、项目配置与 llmanspec 配置的独立 schema（便于表达全局专属字段）。
  - 提供显式命令来写入/修复 YAML LSP schema 头注释。
  - CLI 首次运行时为全局配置自动生成样例文件。
  - 为 CI/本地开发提供 schema 校验入口（just 命令）。
- Non-Goals:
  - 不引入新的配置文件格式（仍为 YAML）。
  - 不为 llmanspec/config.yaml 引入 schema（此变更仅覆盖 llman 用户配置）。
  - 不在每次启动时改写已有配置文件。

## Decisions
- Schema 输出路径
  - 生成到 `artifacts/schema/configs/en/`。
  - 文件名：
    - `llman-config.schema.json`（全局配置）
    - `llman-project-config.schema.json`（项目配置）
    - `llmanspec-config.schema.json`（llmanspec/config.yaml）
- Schema URL 规则
  - 使用 GitHub raw：
    - `https://raw.githubusercontent.com/StrayDragon/llman/main/artifacts/schema/configs/en/llman-config.schema.json`
    - `https://raw.githubusercontent.com/StrayDragon/llman/main/artifacts/schema/configs/en/llman-project-config.schema.json`
    - `https://raw.githubusercontent.com/StrayDragon/llman/main/artifacts/schema/configs/en/llmanspec-config.schema.json`
- Schema 生成方式
  - 复用/抽象现有配置结构，定义 `GlobalConfig`、`ProjectConfig` 与 `LlmanSpecConfig` 三套模型。
  - 使用 `schemars` 生成 JSON schema，字段上写入 `description`/`title`，内容使用英文并与 i18n 术语保持一致。
- 运行时配置校验
  - 配置加载时使用 `jsonschema` 依据对应 schema 校验（全局/项目/llmanspec）。
  - 校验失败直接返回错误，使用 i18n 提示。
- 全局/项目配置差异
  - `skills.dir` 视为全局配置专用字段；项目配置 schema 不包含该字段。
  - 若项目配置出现 `skills.dir`，运行时忽略（防止与全局配置冲突）。
- llmanspec 配置
  - `llmanspec/config.yaml` 由 SDD 生成与读取，schema 仅覆盖 `version`/`locale`/`skills`。
- YAML LSP 头注释
  - 使用 `# yaml-language-server: $schema=...`，并要求出现在文件顶部（如有 `---`，置于其前）。
  - `llman self schema apply` 负责写入/修复头注释，不做其他内容改写。
- 首次样例配置生成
  - CLI 启动时若全局配置缺失，创建样例配置文件并写入 schema 头注释。
  - 样例内容使用 `GlobalConfig::default()` 的序列化输出（允许较完整的默认值），避免额外模板维护成本。
- llmanspec 初始化
  - `llman sdd init` 生成 `llmanspec/config.yaml` 时写入 schema 头注释。
- 校验与开发链路
  - 新增 `llman self schema generate` 生成 schema。
  - 新增 `llman self schema check` 验证 schema 与样例配置（JSON schema 验证），返回非零退出码。
  - 新增 `just check-schemas` 并接入 `just check-all`。

## Risks / Trade-offs
- 首次启动会写入配置文件，可能改变用户目录状态：通过“仅在缺失时创建”降低影响。
- 选择 raw GitHub URL 需要网络访问：符合 LSP 的常规使用方式，但离线时无法自动拉取。
- 全局/项目配置差异化会改变 `skills.dir` 的优先级，需要更新 specs 与测试。

## Migration Plan
- 现有配置保持不变；仅在全局配置缺失时生成样例文件。
- 现有配置如需 schema 头注释，执行 `llman self schema apply` 即可。

## Open Questions
- 是否需要提供“模板化样例配置”（含更多注释）以替代默认序列化输出？
- 是否需要为 llmanspec/config.yaml 引入更丰富的示例（含注释）？
