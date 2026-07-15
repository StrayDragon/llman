# language: zh-CN
# 对应 spec: sdd-ab-evaluation — 评测输出优先安全与质量信号；评测套件含需读编辑格式相关 spec
# 文件的 agentic 任务；支持 multi-style 对比；baseline 预置语义等价；每次 run 产可观测快照；
# --runs N 产聚合报告；可选软评分层；docker runner 支持阿里云镜像。
功能: multi-style 对比、快照、聚合与可选软评分
  场景: 报告优先安全与质量指标
    假如 评测报告生成
    当 检查报告
    那么含质量与安全评分
    而且排在 token/latency 指标之前

  场景: agent 读编辑主 spec 文件
    假如 multi-style agentic 评测运行
    当 agent 执行
    那么读取当前 workspace 的主 spec 文件

  场景: agent 读编辑 delta spec 文件
    假如 评测含某 change
    当 agent 执行
    那么读取该 change 的 delta spec 文件

  场景: 同一任务在三种风格下跑通
    假如 维护者执行 multi-style 评测
    当 评测运行
    那么为三种风格创建隔离 workspace 并分别运行相同任务

  场景: runner 评测前预置 baseline
    假如 一次新评测 run 开始
    当 runner 准备
    那么创建隔离 workspace 并预置语义等价的 baseline specs/changes

  场景: 评测产物含 meta 快照
    假如 一次评测运行结束
    当 检查临时根目录
    那么存在 meta/（或等价）保存快照与日志

  场景: --runs 产聚合报告
    假如 维护者运行 just sdd-claude-style-eval --runs 10（或等价）
    当 评测完成
    那么runner 写出聚合汇总到 meta/aggregate.md（和/或 .json）

  场景: 同时启用硬门禁与可选 rubric
    假如 维护者启用 rubric judge 或 human judge
    当 评测运行
    那么仍以硬门禁为基础
    而且同时输出 rubric/human 评分

  场景: docker 阿里云镜像构建并运行评测
    假如 维护者用传入的 build args（apt/npm/pypi mirror）构建 docker 镜像并运行评测
    当 评测运行
    那么可在容器内完成
