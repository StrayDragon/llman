## 1. Specs
- [x] 1.1 新增 `config-schemas` 规范增量（schema 生成、LSP 头注释、样例配置、校验命令）。
- [x] 1.2 修改 `skills-management` 规范（skills.dir 仅全局配置生效）。
- [x] 1.3 修改 `sdd-workflow` 规范（llmanspec/config.yaml 写入 schema 头注释）。
- [x] 1.4 新增 `tests-ci` 规范增量（check-all 包含 schema 校验）。
- [x] 1.5 运行 `openspec validate add-config-schemas --strict --no-interactive` 并修复问题。
- [x] 1.6 更新 `config-schemas` 规范（运行时 schema 校验）。
- [x] 1.7 运行 `openspec validate add-config-schemas --strict --no-interactive` 并修复问题。

## 2. Implementation (after approval)
- [x] 2.1 定义 `GlobalConfig`/`ProjectConfig` 模型（serde + schemars），补充英文描述。
- [x] 2.2 定义 `LlmanSpecConfig` 模型并生成 schema（serde + schemars），补充英文描述。
- [x] 2.3 新增 `llman self schema` 子命令（generate/apply/check）。
- [x] 2.4 CLI 启动时生成全局样例配置（仅缺失时，含 schema 头）。
- [x] 2.5 `llman sdd init` 写入 `llmanspec/config.yaml` 时附加 schema 头注释。
- [x] 2.6 调整 skills 根目录解析逻辑（忽略本地 config 的 skills.dir），补充测试。
- [x] 2.7 生成并提交 schema 文件到 `artifacts/schema/configs/en/`。
- [x] 2.8 增加 `just check-schemas` 并接入 `check-all`，更新文档。
- [x] 2.9 补充 README/说明中的 schema URL 与用法示例。
- [x] 2.10 配置加载时执行 JSON schema 校验（全局/项目/llmanspec）。
- [x] 2.11 校验失败使用 i18n 提示并更新测试样例配置。
- [x] 2.12 运行 `just check-schemas` 与 `just check-all`。
