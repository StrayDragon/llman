# language: zh-CN
# 对应 spec: config-schemas r125 — 全局配置 schema 支持 skills.repo[]；旧 skills.dir 向后兼容；
# dir 与 repo 同存时 repo 优先并输出 deprecation warning。
# 这些场景由 CLI 子进程驱动（llman self schema check），标 @executable 走 full mode。
功能: 多技能仓库源配置 schema 与向后兼容
  背景:
    假如 llman 二进制已构建

  @req:r125 @executable
  场景: 多 repo 配置通过 schema 校验
    假如 全局 config.yaml 含 multi-repo skills 配置
    当 在非交互终端运行 llman self schema check
    那么 退出码为零

  @req:r125 @executable
  场景: 旧 skills.dir 配置仍校验通过
    假如 全局 config.yaml 含 legacy-dir skills 配置
    当 在非交互终端运行 llman self schema check
    那么 退出码为零

  @req:r125 @executable
  场景: dir 与 repo 同存时 repo 优先并警告
    假如 全局 config.yaml 含 dir-and-repo skills 配置
    当 在非交互终端运行 llman skills
    那么 stderr 包含 deprecated
