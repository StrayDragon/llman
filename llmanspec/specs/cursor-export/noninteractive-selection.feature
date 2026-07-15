# language: zh-CN
# 对应 spec: cursor-export — 非交互 cursor export 选择顺序：db_path > workspace_dir 解析 >
# 自动发现（选最新 workspace 库，优先含 chat/composer 数据）；composer_id 提供时仅导出该会话，
# 否则导出全部；output_mode 仅允许 console/file/single-file（默认 console），非法值报错。
功能: 非交互数据库选择顺序与输出模式校验
  场景: db_path 优先
    假如 非交互模式下设置了 db_path
    当 执行导出
    那么直接使用该数据库路径

  场景: workspace_dir 次之
    假如 设置了 workspace_dir 且未提供 db_path
    当 执行导出
    那么解析 workspace 到其数据库路径并使用

  场景: 无覆盖时自动发现
    假如 既未提供 db_path 也未提供 workspace_dir
    当 执行导出
    那么经自动发现选择数据库路径
    而且优先选含 chat/composer 数据的最新库

  场景: composer_id 提供时仅导出该会话
    假如 非交互模式下设置了 composer_id
    当 执行导出
    那么仅写出匹配的 composer 会话
    而且找不到时返回错误

  场景: 无 composer_id 时导出全部
    假如 非交互模式下未提供 composer_id
    当 执行导出
    那么写出所有可用会话

  场景: 不支持的输出模式报错
    假如 output_mode 不在支持的取值中
    当 命令运行
    那么返回错误并由 CLI 入口呈现
