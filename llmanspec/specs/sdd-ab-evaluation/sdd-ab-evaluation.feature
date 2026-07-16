# language: zh-CN
# managed by llman sdd partition-migrate
功能: sdd-ab-evaluation

  @req:r1
  场景: 报告优先安全与质量指标
    假如 评测报告生成
    当 检查报告
    那么 含质量与安全评分
    而且 排在 token/latency 指标之前

  @req:r1
  场景: agent 读编辑主 spec 文件
    假如 multi-style agentic 评测运行
    当 agent 执行
    那么 读取当前 workspace 的主 spec 文件

  @req:r1
  场景: agent 读编辑 delta spec 文件
    假如 评测含某 change
    当 agent 执行
    那么 读取该 change 的 delta spec 文件

  @req:r1
  场景: 同一任务在三种风格下跑通
    假如 维护者执行 multi-style 评测
    当 评测运行
    那么 为三种风格创建隔离 workspace 并分别运行相同任务

  @req:r1
  场景: runner 评测前预置 baseline
    假如 一次新评测 run 开始
    当 runner 准备
    那么 创建隔离 workspace 并预置语义等价的 baseline specs/changes

  @req:r1
  场景: 评测产物含 meta 快照
    假如 一次评测运行结束
    当 检查临时根目录
    那么 存在 meta/（或等价）保存快照与日志

  @req:r1
  场景: --runs 产聚合报告
    假如 维护者运行 just sdd-claude-style-eval --runs 10（或等价）
    当 评测完成
    那么 runner 写出聚合汇总到 meta/aggregate.md（和/或 .json）

  @req:r1
  场景: 同时启用硬门禁与可选 rubric
    假如 维护者启用 rubric judge 或 human judge
    当 评测运行
    那么 仍以硬门禁为基础
    而且 同时输出 rubric/human 评分

  @req:r1
  场景: docker 阿里云镜像构建并运行评测
    假如 维护者用传入的 build args（apt/npm/pypi mirror）构建 docker 镜像并运行评测
    当 评测运行
    那么 可在容器内完成

  @req:r1
  场景: 在共享场景集上运行对比评测
    假如 用户对目标场景集执行评测流程
    当 评测运行
    那么 系统在等价输入上运行 legacy 与新风格生成/评测

  @req:r1
  场景: 评测在隔离配置目录下运行
    假如 维护者运行评测脚本并从 agentdev/promptfoo/ 读取 fixtures 与模型列表
    当 评测运行
    那么 Promptfoo 数据写入该流程创建的临时工作目录

  @req:r1
  场景: 评测套件位于 agentdev
    假如 维护者在仓库查找 promptfoo 评测套件位置
    当 查找
    那么 评测套件位于 agentdev/promptfoo/

  @req:r1
  场景: 运行一次 Claude Code agentic 评测
    假如 维护者运行 Promptfoo fixture 驱动 Claude Code agent 完成一轮 SDD authoring + validate
    当 评测运行
    那么 输出含 results.json 与 results.html
