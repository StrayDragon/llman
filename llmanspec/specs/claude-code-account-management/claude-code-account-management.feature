# language: zh-CN
# managed by llman sdd partition-migrate
功能: claude-code-account-management

  @req:r1
  场景: edit 命令尝试打开配置文件
    假如 用户运行 llman x claude-code account edit
    当 命令执行
    那么 尝试在编辑器中打开 Claude Code 配置文件

  @req:r1
  场景: VISUAL 优先于 EDITOR
    假如 $VISUAL 设为 nvim 且 $EDITOR 设为 code --wait
    当 打开配置文件
    那么 使用 nvim 打开

  @req:r1
  场景: 编辑器含参数时正确执行
    假如 $EDITOR 设为 code --wait 且用户运行 llman x claude-code account edit
    当 命令执行
    那么 执行 code --wait <claude-code.toml-path>
    而且 若编辑器非零退出则返回错误

  @req:r1
  场景: LLMAN_CONFIG_DIR 覆盖配置路径
    假如 LLMAN_CONFIG_DIR 设为 {override_dir} 且用户运行 llman x claude-code account edit
    当 命令执行
    那么 打开 {override_dir}/claude-code.toml

  @req:r1
  场景: 首次编辑创建模板
    假如 <config-dir>/claude-code.toml 不存在且用户运行 llman x claude-code account edit
    当 命令执行
    那么 创建目录并写入最小模板
    而且 以该路径启动编辑器

  @req:r1
  场景: 编辑器返回失败时报错
    假如 选定编辑器以状态码 2 退出
    当 llman x claude-code account edit 执行
    那么 返回错误并指明编辑器退出状态

  @req:r1
  场景: x cc 别名等价
    假如 用户运行 llman x cc account edit
    当 命令执行
    那么 行为与 llman x claude-code account edit 完全一致

  @req:r1
  场景: 非 Windows 输出 POSIX export
    假如 用户在非 Windows 平台运行 llman x claude-code account env {group}
    而且 该组含 FOO=bar
    当 命令执行
    那么 stdout 含 export FOO='bar'

  @req:r1
  场景: Windows 输出 PowerShell env
    假如 用户在 Windows 运行 llman x claude-code account env {group}
    而且 该组含 FOO=bar
    当 命令执行
    那么 stdout 含 $env:FOO='bar'

  @req:r1
  场景: 键名升序稳定输出
    假如 组含 B=2 与 A=1
    当 命令执行
    那么 stdout 行按 A 然后 B 顺序输出

  @req:r1
  场景: 非法键名时报错且不输出注入语句
    假如 组含非法键名 BAD-KEY=1
    当 命令执行
    那么 非零退出
    而且 stdout 不含注入语句

  @req:r1
  场景: 组不存在时报错
    假如 用户运行 llman x claude-code account env {missing_group}
    当 命令执行
    那么 非零退出并报告该组不存在

  @req:r1
  场景: account list 对敏感值脱敏
    假如 配置组含 DB_PASSWORD={secret}
    当 用户运行 llman x claude-code account list
    那么 输出中该值被脱敏
    而且 不包含完整明文
