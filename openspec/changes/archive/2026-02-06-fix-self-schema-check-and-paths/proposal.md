## Why
`llman self schema` 属于开发者工具链的关键环节：生成/应用 schema 头注释、以及校验 schema 与样例配置的一致性。当前存在两类问题：
- `schema check` 仅用 default struct 作为样例校验，无法覆盖真实 YAML 文件（例如用户已改动配置但仍需要校验）。
- `schema apply` 的 project/llmanspec config path 直接基于 `current_dir` 拼接，若在 repo 子目录运行，会对错误路径尝试写入/提示。
- schema header 应用策略会移除所有 `# yaml-language-server: $schema=` 行，可能过于激进（误删用户多文档/多 header 的合法内容）。

### Current Behavior（基于现有代码）
- `run_check`：只对 `GlobalConfig::default()`/`ProjectConfig::default()`/`SddConfig::default()` 做 JSON schema validate（`src/self_command.rs`）。
- `project_config_path`/`llmanspec_config_path`：`env::current_dir()?.join(...)`（`src/config_schema.rs`）。
- `apply_schema_header_to_content`：删除所有匹配行后再插入（`src/config_schema.rs`）。

## What Changes
- `schema check`：优先使用真实配置文件作为样例（global/project/llmanspec），缺失时再回退 default 实例。
- `schema apply`：通过 root discovery（向上查找 repo/config 根）定位 `.llman/config.yaml` 与 `llmanspec/config.yaml`，不再假设 `cwd` 是根。
- schema header 应用：更“最小侵入”——确保顶部只有一个有效 header，不删除不相关内容。

### Non-Goals（边界）
- 不改变 schema 文件输出位置与 URL 常量。
- 不引入新的配置文件格式或 schema 草案版本。

## Impact
- Affected specs: `specs/config-schemas/spec.md`
- Affected code: `src/self_command.rs`, `src/config_schema.rs`
