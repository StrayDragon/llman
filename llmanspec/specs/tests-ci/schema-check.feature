# language: zh-CN
# 对应 spec: tests-ci — just check-all MUST 包含 schema 校验步骤，
# 确保生成的 JSON schema 与样例配置有效且可用。
功能: check-all 包含 schema 校验
  @req:r1
  场景: check-all 执行 schema 校验
    假如 开发者运行 just check-all
    当 check-all 执行其步骤序列
    那么 just check-schemas 会被执行
