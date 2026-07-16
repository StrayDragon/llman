# language: zh-CN
# managed by llman sdd partition-migrate
功能: codex-agents-management

  @req:r1
  场景: 查看帮助含全部子命令
    假如 用户运行 llman x codex agents --help
    当 命令执行
    那么 帮助含 import / sync / inject / status 子命令说明

  @req:r1
  场景: status 不落盘
    假如 用户运行 llman x codex agents status
    当 命令执行
    那么 不写入任何文件
    而且 退出码为 0

  @req:r1
  场景: sync dry-run 展示计划不落盘
    假如 用户运行 llman x codex agents sync --dry-run
    当 命令执行
    那么 输出将执行的同步操作列表
    而且 不修改目标目录

  @req:r1
  场景: 非交互未确认则失败
    假如 在非交互环境运行 sync 会写文件但未提供 --yes/--force
    当 命令执行
    那么 退出并提示需要 --yes/--force 或使用 --dry-run

  @req:r1
  场景: 默认托管目录
    假如 用户运行 sync 且未指定 --managed-dir
    当 命令执行
    那么 从 $LLMAN_CONFIG_DIR/codex/agents/ 读取 *.toml 并同步

  @req:r1
  场景: 使用 agents-dir 指定目标
    假如 用户运行 sync --agents-dir {target_dir}
    当 命令执行
    那么 将输出同步到 {target_dir}

  @req:r1
  场景: 导入全部文件
    假如 目标 agents 目录含 a.toml 与 b.toml，用户运行 import
    当 命令执行
    那么 托管目录生成/更新 a.toml 与 b.toml

  @req:r1
  场景: 仅导入指定文件
    假如 目标 agents 目录含 a.toml 与 b.toml，用户运行 import --only a
    当 命令执行
    那么 托管目录仅生成/更新 a.toml
    而且 不导入 b.toml

  @req:r1
  场景: 创建 symlink
    假如 托管目录存在 defaults.toml 且目标无该文件，用户运行 sync
    当 命令执行
    那么 目标目录出现 defaults.toml
    而且 其为指向托管文件的 symlink

  @req:r1
  场景: copy 模式同步
    假如 用户运行 sync --mode copy
    当 命令执行
    那么 目标目录的 *.toml 为常规文件（非 symlink）
    而且 内容与托管目录一致

  @req:r1
  场景: sync 覆盖产生备份
    假如 目标目录已存在普通文件 a.toml 且将被同步替换
    当 命令执行
    那么 目标目录产生 a.toml.llman.bak.<timestamp>
    而且 将 a.toml 更新为同步结果

  @req:r1
  场景: inject 注入新 marker 区块
    假如 托管的 reviewer.toml 含 developer_instructions 且无 marker
    当 用户运行 inject --template {tpl}
    那么 developer_instructions 内含 marker 区块
    而且 含 ## llman prompts: {tpl} 段落
