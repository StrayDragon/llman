## Why
- `~/.config/llman/config.yaml` 目前缺少 YAML schema，LSP 无法提供补全/提示，用户容易写错字段。
- 配置文件可能不存在，用户缺少可参考的样例，且当前没有统一的 schema 生成与校验链路。

## What Changes
- 为 llman 配置引入自动生成的 JSON schema，并存放到 `artifacts/schema/configs/en/`。
  - 全局配置 schema：`llman-config.schema.json`
  - 项目配置 schema：`llman-project-config.schema.json`
  - llmanspec 配置 schema：`llmanspec-config.schema.json`
- 新增 `llman self schema` 管理命令，用于：生成/校验 schema、为 YAML 写入 LSP schema 头注释（含 llmanspec 配置）。
- CLI 启动时若全局配置缺失，自动创建带 schema 头注释的样例配置（默认值或示例内容）。
- `llman sdd init` 生成 `llmanspec/config.yaml` 时写入 schema 头注释。
- 增加 `just check-schemas` 并将其接入 `just check-all`。
- 调整 skills 根目录读取逻辑：`skills.dir` 仅从全局配置读取，本地配置 `.llman/config.yaml` 不再覆盖。
- 配置加载时进行 JSON schema 校验（全局/项目/llmanspec），不匹配时返回本地化错误。

## Impact
- Specs：新增 `config-schemas`；修改 `skills-management` 与 `sdd-workflow`；新增 `tests-ci` 的 schema 检查要求。
- Code：新增配置模型与 schema 生成；新增 CLI 子命令；启动时写入样例配置；更新 skills 路径解析；更新 sdd init 配置写入。
- Code：配置加载路径增加 JSON schema 校验与错误提示。
- Docs：README/说明补充 schema 使用方式与 raw URL。
