# language: zh-CN
# 对应 spec: claude-code-account-management — CLI MUST 提供 llman x claude-code account edit；
# 编辑器选择 VISUAL > EDITOR > vi；MUST 支持编辑器命令含参数；配置路径遵循 LLMAN_CONFIG_DIR；
# 缺失文件时创建最小模板；编辑器非零退出时报错；x cc 别名等价。
功能: Claude Code account edit 命令与编辑器选择
  @req:r11
  场景: edit 命令尝试打开配置文件
    假如 用户运行 llman x claude-code account edit
    当 命令执行
    而且 那么尝试在编辑器中打开 Claude Code 配置文件

  @req:r11
  场景: VISUAL 优先于 EDITOR
    假如 $VISUAL 设为 nvim 且 $EDITOR 设为 code --wait
    当 打开配置文件
    而且 那么使用 nvim 打开

  @req:r11
  场景: 编辑器含参数时正确执行
    假如 $EDITOR 设为 code --wait 且用户运行 llman x claude-code account edit
    当 命令执行
    而且 那么执行 code --wait <claude-code.toml-path>
    而且 而且若编辑器非零退出则返回错误

  @req:r11
  场景: LLMAN_CONFIG_DIR 覆盖配置路径
    假如 LLMAN_CONFIG_DIR 设为 {override_dir} 且用户运行 llman x claude-code account edit
    当 命令执行
    而且 那么打开 {override_dir}/claude-code.toml

  @req:r11
  场景: 首次编辑创建模板
    假如 <config-dir>/claude-code.toml 不存在且用户运行 llman x claude-code account edit
    当 命令执行
    而且 那么创建目录并写入最小模板
    而且 而且以该路径启动编辑器

  @req:r11
  场景: 编辑器返回失败时报错
    假如 选定编辑器以状态码 2 退出
    当 llman x claude-code account edit 执行
    而且 那么返回错误并指明编辑器退出状态

  @req:r11
  场景: x cc 别名等价
    假如 用户运行 llman x cc account edit
    当 命令执行
    而且 那么行为与 llman x claude-code account edit 完全一致
