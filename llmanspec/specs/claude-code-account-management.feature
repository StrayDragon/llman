# language: zh-CN
# capability: claude-code-account-management
# purpose: 规范 `llman x claude-code account` 的编辑入口、env 注入输出与敏感值脱敏行为。
# scope: llmanspec/specs/claude-code-account-management.feature

功能: claude-code-account-management

  @req:r11
  规则: Claude Code account edit 命令与编辑器选择
    对应 spec: claude-code-account-management — CLI MUST 提供 llman x claude-code account edit； 编辑器选择 VISUAL > EDITOR > vi；MUST 支持编辑器命令含参数；配置路径遵循 LLMAN_CONFIG_DIR； 缺失文件时创建最小模板；编辑器非零退出时报错；x cc 别名等价。

    场景: account-list-empty-config-guidance
      当 运行 llman x claude-code account list
      那么 退出码为零
      那么 stdout 包含 account import
  @req:r40
  规则: account env 注入输出与 account list 敏感值脱敏
    account list 展示敏感环境变量值时 MUST 脱敏。

    场景: account-list-masks-secret-values
      假如 含配置组的 claude-code 配置
      当 运行 llman x claude-code account list
      那么 退出码为零
      那么 stdout 不含 sk-secret-2
