# language: zh-CN
# capability: codex-agents-management
# purpose: 规范 `llman x codex agents` 命令组对 Codex custom agents 配置的导入、同步与注入行为。
# scope: llmanspec/specs/codex-agents-management

功能: codex-agents-management

  @req:r16 @human
  场景: codex agents 命令组、托管目录与确认门禁
    - 对应 spec: codex-agents-management — 系统 MUST 提供 llman x codex agents 命令组 （import/sync/inject/status）；status 只读；支持 --dry-run；非交互写操作需 --yes/--force； 交互向导收集参数；llman 托管目录为 source of truth；目标目录可解析可覆盖。

  @req:r45 @human
  场景: import/sync/inject 的文件操作与冲突备份
    - The system MUST satisfy the harness scenarios for `import/sync/inject 的文件操作与冲突备份`: 对应 spec: codex-agents-management — import 将目标 *.toml 纳入托管目录（支持 --only）； sync 默认逐文件 symlink（支持 --mode copy）；冲突时先备份再覆盖（.llman.bak.<timestamp>）； inject 将 prompts 模板注入 developer_instructions（marker 幂等更新）。
