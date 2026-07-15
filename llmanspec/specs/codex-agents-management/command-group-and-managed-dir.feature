# language: zh-CN
# 对应 spec: codex-agents-management — 系统 MUST 提供 llman x codex agents 命令组
# （import/sync/inject/status）；status 只读；支持 --dry-run；非交互写操作需 --yes/--force；
# 交互向导收集参数；llman 托管目录为 source of truth；目标目录可解析可覆盖。
功能: codex agents 命令组、托管目录与确认门禁
  场景: 查看帮助含全部子命令
    假如 用户运行 llman x codex agents --help
    当 命令执行
    那么帮助含 import / sync / inject / status 子命令说明

  场景: status 不落盘
    假如 用户运行 llman x codex agents status
    当 命令执行
    那么不写入任何文件
    而且退出码为 0

  场景: sync dry-run 展示计划不落盘
    假如 用户运行 llman x codex agents sync --dry-run
    当 命令执行
    那么输出将执行的同步操作列表
    而且不修改目标目录

  场景: 非交互未确认则失败
    假如 在非交互环境运行 sync 会写文件但未提供 --yes/--force
    当 命令执行
    那么退出并提示需要 --yes/--force 或使用 --dry-run

  场景: 默认托管目录
    假如 用户运行 sync 且未指定 --managed-dir
    当 命令执行
    那么从 $LLMAN_CONFIG_DIR/codex/agents/ 读取 *.toml 并同步

  场景: 使用 agents-dir 指定目标
    假如 用户运行 sync --agents-dir {target_dir}
    当 命令执行
    那么将输出同步到 {target_dir}
