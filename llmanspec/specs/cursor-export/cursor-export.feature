# language: zh-CN
# managed by llman sdd partition-migrate
功能: cursor-export

  @req:r1
  场景: 排序展示不影响导出正确性
    假如 交互 UI 展示的是排序后的对话列表
    当 用户选择了 A 与 B
    那么 即使底层存储顺序不同，导出也只包含 A 与 B

  @req:r1
  场景: 搜索选择导出正确
    假如 用户通过 search more 流程选择了若干对话
    当 执行导出
    那么 导出结果与搜索选择完全一致

  @req:r1
  场景: 大库仅选少量对话时只加载选中项
    假如 workspace 中存在大量对话但用户只选择少量导出
    当 执行导出
    那么 仅为被选中对话加载完整内容

  @req:r1
  场景: db_path 优先
    假如 非交互模式下设置了 db_path
    当 执行导出
    那么 直接使用该数据库路径

  @req:r1
  场景: workspace_dir 次之
    假如 设置了 workspace_dir 且未提供 db_path
    当 执行导出
    那么 解析 workspace 到其数据库路径并使用

  @req:r1
  场景: 无覆盖时自动发现
    假如 既未提供 db_path 也未提供 workspace_dir
    当 执行导出
    那么 经自动发现选择数据库路径
    而且 优先选含 chat/composer 数据的最新库

  @req:r1
  场景: composer_id 提供时仅导出该会话
    假如 非交互模式下设置了 composer_id
    当 执行导出
    那么 仅写出匹配的 composer 会话
    而且 找不到时返回错误

  @req:r1
  场景: 无 composer_id 时导出全部
    假如 非交互模式下未提供 composer_id
    当 执行导出
    那么 写出所有可用会话

  @req:r1
  场景: 不支持的输出模式报错
    假如 output_mode 不在支持的取值中
    当 命令运行
    那么 返回错误并由 CLI 入口呈现

  @req:r1
  场景: single-file 模式输出位置
    假如 output_mode 为 single-file
    当 执行导出
    那么 写入 output_file（若提供），否则默认 cursor_conversations.md

  @req:r1
  场景: file 模式输出位置
    假如 output_mode 为 file
    当 执行导出
    那么 写入 output_file 目录（若提供），否则默认 ./cursor_exports

  @req:r1
  场景: SQLite 连接使用 FULL_MUTEX
    假如 cursor export 打开数据库
    当 SQLite 连接创建
    那么 使用 FULL_MUTEX
    而且 或在注释中明确说明 NO_MUTEX 为单线程安全

  @req:r1
  场景: 数据库可正常打开时导出无退化
    假如 数据库可正常打开
    当 cursor export 执行导出
    那么 功能正常
    而且 无退化
