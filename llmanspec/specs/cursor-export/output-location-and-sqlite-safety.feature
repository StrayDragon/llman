# language: zh-CN
# 对应 spec: cursor-export — 文件输出模式遵循确定性位置与命名；SQLite 连接 MUST 使用
# FULL_MUTEX（或线程安全等价标志），MUST NOT 使用 NO_MUTEX 除非经安全分析确认单线程并加注释。
功能: 文件输出位置规则与 SQLite 线程安全
  @req:r3
  场景: single-file 模式输出位置
    假如 output_mode 为 single-file
    当 执行导出
    而且 那么写入 output_file（若提供），否则默认 cursor_conversations.md

  @req:r3
  场景: file 模式输出位置
    假如 output_mode 为 file
    当 执行导出
    而且 那么写入 output_file 目录（若提供），否则默认 ./cursor_exports

  @req:r3
  场景: SQLite 连接使用 FULL_MUTEX
    假如 cursor export 打开数据库
    当 SQLite 连接创建
    而且 那么使用 FULL_MUTEX
    而且 而且或在注释中明确说明 NO_MUTEX 为单线程安全

  @req:r3
  场景: 数据库可正常打开时导出无退化
    假如 数据库可正常打开
    当 cursor export 执行导出
    而且 那么功能正常
    而且 而且无退化
