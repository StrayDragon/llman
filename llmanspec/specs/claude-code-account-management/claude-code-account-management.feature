# language: zh-CN
# capability: claude-code-account-management
# purpose: 规范 `llman x claude-code account` 的编辑入口、env 注入输出与敏感值脱敏行为。
# scope: llmanspec/specs/claude-code-account-management

功能: claude-code-account-management

  @req:r11 @human
  场景: Claude Code account edit 命令与编辑器选择
    - 对应 spec: claude-code-account-management — CLI MUST 提供 llman x claude-code account edit； 编辑器选择 VISUAL > EDITOR > vi；MUST 支持编辑器命令含参数；配置路径遵循 LLMAN_CONFIG_DIR； 缺失文件时创建最小模板；编辑器非零退出时报错；x cc 别名等价。

  @req:r40 @human
  场景: account env 注入输出与 account list 敏感值脱敏
    - account list 展示敏感环境变量值时 MUST 脱敏。
