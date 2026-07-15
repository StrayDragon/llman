# language: zh-CN
# 对应 spec: sdd-openspec-interop — 迁移范围含 specs/active changes/archive；同名冲突即失败
# 且不覆盖；非标准目录输出 warning 并按相对路径复制；迁移成功后交互式默认不删除旧目录；
# export 补齐 OpenSpec 元数据，import 补齐 valid_scope。
功能: 迁移范围、冲突策略、旧目录删除与元数据补齐
  场景: 目标同名冲突即失败
    假如 目标目录中已存在将写入的同名文件
    当 命令运行
    那么返回非零并中止
    而且不覆盖
    而且不跳过冲突文件继续

  场景: 检测到非标准目录输出 warning
    假如 导入源包含 {nonstandard_dir}
    当 命令运行
    那么输出 warning 说明检测到非标准目录
    而且按相对路径复制到目标侧

  场景: 交互模式默认不删除旧目录
    假如 迁移执行成功且系统进入删除确认提示
    当 展示默认选项
    那么默认选项为不删除

  场景: 用户确认删除旧目录
    假如 迁移执行成功且用户在交互提示中明确选择删除
    当 系统处理
    那么删除旧迁移目录

  场景: 非交互模式不删除旧目录
    假如 用户在非交互模式下运行 import/export
    当 命令运行
    那么不执行旧目录删除操作

  场景: 导出自动补齐 OpenSpec 元数据
    假如 用户执行 llman sdd export --style openspec 且目标缺失 openspec/config.yaml
    当 执行写入阶段
    那么创建 openspec/config.yaml（至少含 schema: spec-driven）
    而且为 active change 补齐 .openspec.yaml

  场景: 导入补齐 valid_scope
    假如 导入源 spec 缺失 valid_scope
    当 执行写入阶段
    那么为目标 spec 补齐必需的 valid_scope
