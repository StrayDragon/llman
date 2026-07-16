# language: zh-CN
# 对应 spec: sdd-openspec-interop — 系统 MUST 提供 import/export --style openspec 双向互转；
# --style 必填且仅允许 openspec；import/export MUST 默认先 dry-run，交互终端双确认后写入，
# 非交互拒绝写入返回非零。
功能: OpenSpec 双向互转命令与安全门禁
  @req:r1
  场景: 导入命令参数有效
    假如 用户执行 llman sdd import --style openspec
    当 命令运行
    而且 那么开始构建从 openspec/ 到 llmanspec/ 的迁移计划

  @req:r1
  场景: 导入非法 style 被拒
    假如 用户执行 llman sdd import --style {bad_style}
    当 命令运行
    而且 那么返回非零并提示仅支持 openspec

  @req:r1
  场景: 导出命令参数有效
    假如 用户执行 llman sdd export --style openspec
    当 命令运行
    而且 那么开始构建从 llmanspec/ 到 openspec/ 的迁移计划

  @req:r1
  场景: 导出非法 style 被拒
    假如 用户执行 llman sdd export --style {bad_style}
    当 命令运行
    而且 那么返回非零并提示仅支持 openspec

  @req:r1
  场景: 交互环境双确认后执行写入
    假如 用户在交互终端执行 llman sdd import --style openspec
    当 依次通过两次确认
    而且 那么命令执行迁移写入

  @req:r1
  场景: 非交互环境拒绝执行
    假如 用户在非交互环境执行 llman sdd export --style openspec
    当 命令运行
    而且 那么输出迁移计划后返回非零
