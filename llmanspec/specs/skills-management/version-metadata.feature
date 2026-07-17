# language: zh-CN
# 对应 spec: skills-management — Skills 模板含 metadata.version（与生成它的 CLI 版本一致）；
# init 与 update-skills 自动填充当前 CLI 版本；缺失 version 不阻断加载；主版本不匹配输出警告但不阻断。
功能: 技能版本元数据与不匹配警告
  @req:r1
  场景: init 填充版本
    假如 用户运行 llman sdd init
    当 生成 skills
    那么 metadata.version 为当前 CLI 版本

  @req:r1
  场景: update-skills 同步版本
    假如 用户运行 llman sdd update-skills
    当 更新完成
    而且 那么更新后的 skills 的 metadata.version 为当前 CLI 版本

  场景: 缺失 version 不阻断
    假如 现有 skill 无 metadata.version
    当 skill 加载
    而且 那么正常加载
    而且 而且视为未版本化

  @req:r1
  场景: 主版本不匹配警告
    假如 skill 的 metadata.version 与当前 CLI 主版本（major.minor）不一致
    当 加载时
    而且 那么输出版本不匹配警告
    而且 而且不阻断 skill 加载或执行
