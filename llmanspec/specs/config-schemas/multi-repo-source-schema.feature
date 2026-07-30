# language: zh-CN
# 对应 spec: config-schemas r125 — 全局配置 schema 仅支持 skills.repo[]；旧 skills.dir
# MUST 被 schema 拒绝；缺失或非目录的 repo path MUST 在启动时警告并过滤。
# 这些场景由 CLI 子进程驱动（llman self schema check / llman skills），标 @executable 走 full mode。
功能: 多技能仓库源配置 schema
  背景:
    假如 llman 二进制已构建

  @req:r125 @executable
  场景: 多 repo 配置通过 schema 校验
    假如 全局 config.yaml 含 multi-repo skills 配置
    当 在非交互终端运行 llman self schema check
    那么 退出码为零

  @req:r125 @executable
  场景: 旧 skills.dir 配置 schema 校验失败
    假如 全局 config.yaml 含 legacy-dir skills 配置
    当 在非交互终端运行 llman self schema check
    那么 退出码非零

  @req:r125 @executable
  场景: 缺失 repo 路径启动时警告并过滤
    假如 全局 config.yaml 含 missing-path skills 配置
    当 在非交互终端运行 llman skills
    那么 stderr 包含 skipping
