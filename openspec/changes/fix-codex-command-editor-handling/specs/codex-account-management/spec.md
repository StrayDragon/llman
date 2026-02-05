## ADDED Requirements
### Requirement: 编辑器命令必须支持参数
当打开配置文件进行编辑时，Codex account manager MUST 支持 `$VISUAL` 或 `$EDITOR` 包含参数（例如 `code --wait`）。实现 MUST 执行解析后的命令，并将目标配置路径作为最后一个参数追加。

#### Scenario: editor 包含参数
- **WHEN** `$EDITOR` 设置为 `code --wait` 且用户运行 `llman x codex account edit <name>`
- **THEN** 命令执行 `code --wait <config-path>`；若编辑器以非零退出则返回错误
