# language: zh-CN
# 对应 spec: skills-management — 按 agent 目标启用/禁用技能（link 模式创建软链接，skip 跳过）；
# 断链 symlink 视为已存在条目；冲突提示取消是安全 abort；非交互冲突需 --target-conflict 策略；
# 启用状态实时计算不依赖 registry。
功能: 链接启用/禁用、冲突处理与实时状态
  @req:r80
  场景: 为单个 agent 禁用技能 link
    假如 用户禁用某技能在 mode=link 的目标下
    当 管理器执行
    而且 那么仅移除该目标下的软链接
    而且 而且源目录保持不变

  @req:r80
  场景: 目标路径非法
    假如 目标路径存在但不是目录
    当 管理器执行
    而且 那么记录错误
    而且 而且不创建链接

  @req:r80
  场景: 交互模式下 link 目标冲突
    假如 交互模式下 mode=link 目标已存在同名条目且非期望软链接
    当 管理器执行
    而且 那么提示用户选择覆盖或跳过
    而且 而且覆盖时删除后重建链接

  @req:r80
  场景: 非交互冲突无策略报错
    假如 非交互模式下冲突且未传 --target-conflict
    当 命令执行
    而且 那么返回错误并提示使用 --target-conflict=overwrite|skip

  @req:r80
  场景: 覆盖断链 symlink
    假如 某 target 存在名为 skill_id 的断链 symlink 且用户希望启用该 skill
    当 冲突处理流程运行
    而且 那么可按选定冲突策略覆盖该断链 symlink

  @req:r80
  场景: 移除断链 symlink
    假如 某 target 存在名为 skill_id 的断链 symlink 且用户希望禁用该 skill
    当 管理器执行
    而且 那么该断链 symlink 被移除

  @req:r80
  场景: 取消冲突提示是安全 abort
    假如 target 存在冲突条目且用户在 overwrite/skip 提示中取消
    当 管理器处理
    而且 那么整体 abort 且不应用任何变更
    而且 而且以成功状态退出

  @req:r80
  场景: 交互默认来自文件系统
    假如 交互模式选择某 target
    当 计算默认勾选
    而且 那么基于目标目录真实链接状态计算

  @req:r80
  场景: 非交互已链接优先
    假如 非交互模式下某技能在某 target 已存在正确链接
    当 管理器执行
    而且 那么保持该技能启用
    而且 而且不因配置默认值覆盖

  @req:r80
  场景: 运行不写持久化状态文件
    假如 管理器完成交互或非交互会话
    当 会话结束
    而且 那么不创建或更新任何 registry 状态文件
