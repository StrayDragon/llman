# language: zh-CN
# 对应 spec: agent-tools-usage-stats — 缺失 token 字段视为 unknown 不估算；数据源本地只读无网络；
# Claude Code sidechain 默认含且单独计数；known-token 覆盖率被报告；Codex breakdown 可选默认关；
# 工具特定 flag 稳定；长扫描提供进度反馈；TUI 提供交互过滤表单。
功能: token 处理、sidechain、breakdown、TUI 与数据源安全
  @req:r3
  场景: 未知 token 不破坏聚合
    假如 过滤数据集中部分会话无 token 信息
    当 命令执行
    而且 那么仍成功渲染所请求视图

  @req:r3
  场景: 离线执行
    假如 机器无网络连接
    当 用户运行 llman x codex stats
    而且 那么仍用本地状态运行

  @req:r3
  场景: sidechain 可见且计入总量
    假如 某 Claude Code 项目含主会话与带 token 的关联 sidechain
    当 查看 sessions 视图
    而且 那么列出两个会话

  @req:r3
  场景: 覆盖率可见
    假如 数据集含缺失 token 信息的会话
    当 生成报告
    而且 那么含表示 known-token 覆盖率的字段

  @req:r3
  场景: 默认仅用 thread tokens_used
    假如 用户运行 llman x codex stats 且不带 breakdown 选项
    当 命令执行
    而且 那么不要求解析 rollout JSONL 即可产出总量

  @req:r3
  场景: no-sidechain 禁用 sidechain
    假如 用户运行 llman x claude-code stats --no-sidechain
    当 命令执行
    而且 那么sidechain 会话被排除出所有视图

  @req:r3
  场景: codex breakdown 在 TUI 显示进度
    假如 用户运行 llman x codex stats --tui --with-breakdown 且解析多个 rollout 文件
    当 扫描执行
    而且 那么TUI 显示持续更新的进度指示器直到完成

  @req:r3
  场景: TUI 过滤更新视图
    假如 用户在 TUI 过滤表单修改时间范围并提交
    当 重新扫描
    而且 那么显示的 sessions/trend 更新以反映新范围
