# language: zh-CN
# managed by llman sdd partition-migrate
功能: sdd-openspec-interop

  @req:r1
  场景: 导入命令参数有效
    假如 用户执行 llman sdd import --style openspec
    当 命令运行
    那么 开始构建从 openspec/ 到 llmanspec/ 的迁移计划

  @req:r1
  场景: 导入非法 style 被拒
    假如 用户执行 llman sdd import --style {bad_style}
    当 命令运行
    那么 返回非零并提示仅支持 openspec

  @req:r1
  场景: 导出命令参数有效
    假如 用户执行 llman sdd export --style openspec
    当 命令运行
    那么 开始构建从 llmanspec/ 到 openspec/ 的迁移计划

  @req:r1
  场景: 导出非法 style 被拒
    假如 用户执行 llman sdd export --style {bad_style}
    当 命令运行
    那么 返回非零并提示仅支持 openspec

  @req:r1
  场景: 交互环境双确认后执行写入
    假如 用户在交互终端执行 llman sdd import --style openspec
    当 依次通过两次确认
    那么 命令执行迁移写入

  @req:r1
  场景: 非交互环境拒绝执行
    假如 用户在非交互环境执行 llman sdd export --style openspec
    当 命令运行
    那么 输出迁移计划后返回非零

  @req:r1
  场景: 目标同名冲突即失败
    假如 目标目录中已存在将写入的同名文件
    当 命令运行
    那么 返回非零并中止
    而且 不覆盖
    而且 不跳过冲突文件继续

  @req:r1
  场景: 检测到非标准目录输出 warning
    假如 导入源包含 {nonstandard_dir}
    当 命令运行
    那么 输出 warning 说明检测到非标准目录
    而且 按相对路径复制到目标侧

  @req:r1
  场景: 交互模式默认不删除旧目录
    假如 迁移执行成功且系统进入删除确认提示
    当 展示默认选项
    那么 默认选项为不删除

  @req:r1
  场景: 用户确认删除旧目录
    假如 迁移执行成功且用户在交互提示中明确选择删除
    当 系统处理
    那么 删除旧迁移目录

  @req:r1
  场景: 非交互模式不删除旧目录
    假如 用户在非交互模式下运行 import/export
    当 命令运行
    那么 不执行旧目录删除操作

  @req:r1
  场景: 导出自动补齐 OpenSpec 元数据
    假如 用户执行 llman sdd export --style openspec 且目标缺失 openspec/config.yaml
    当 执行写入阶段
    那么 创建 openspec/config.yaml（至少含 schema: spec-driven）
    而且 为 active change 补齐 .openspec.yaml

  @req:r1
  场景: 导入补齐 valid_scope
    假如 导入源 spec 缺失 valid_scope
    当 执行写入阶段
    那么 为目标 spec 补齐必需的 valid_scope
