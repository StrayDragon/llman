# language: zh-CN
# 对应 spec: codex-agents-management — import 将目标 *.toml 纳入托管目录（支持 --only）；
# sync 默认逐文件 symlink（支持 --mode copy）；冲突时先备份再覆盖（.llman.bak.<timestamp>）；
# inject 将 prompts 模板注入 developer_instructions（marker 幂等更新）。
功能: import/sync/inject 的文件操作与冲突备份
  场景: 导入全部文件
    假如 目标 agents 目录含 a.toml 与 b.toml，用户运行 import
    当 命令执行
    那么托管目录生成/更新 a.toml 与 b.toml

  场景: 仅导入指定文件
    假如 目标 agents 目录含 a.toml 与 b.toml，用户运行 import --only a
    当 命令执行
    那么托管目录仅生成/更新 a.toml
    而且不导入 b.toml

  场景: 创建 symlink
    假如 托管目录存在 defaults.toml 且目标无该文件，用户运行 sync
    当 命令执行
    那么目标目录出现 defaults.toml
    而且其为指向托管文件的 symlink

  场景: copy 模式同步
    假如 用户运行 sync --mode copy
    当 命令执行
    那么目标目录的 *.toml 为常规文件（非 symlink）
    而且内容与托管目录一致

  场景: sync 覆盖产生备份
    假如 目标目录已存在普通文件 a.toml 且将被同步替换
    当 命令执行
    那么目标目录产生 a.toml.llman.bak.<timestamp>
    而且将 a.toml 更新为同步结果

  场景: inject 注入新 marker 区块
    假如 托管的 reviewer.toml 含 developer_instructions 且无 marker
    当 用户运行 inject --template {tpl}
    那么developer_instructions 内含 marker 区块
    而且含 ## llman prompts: {tpl} 段落
