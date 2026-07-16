# language: zh-CN
# 对应 spec: sdd-eval-workflow-dsl — playbook MUST 为 YAML 文档，定义 workflow/jobs/steps 模型；
# variants 按稳定 id 寻址并可经 matrix 展开；job needs 依赖确定性解析（拓扑序 + 声明序兜底）。
功能: playbook 采用 workflow/jobs/steps 模型与确定性依赖
  @req:r1
  场景: 缺失 workflow.jobs 时显式失败
    假如 用户运行 llman x sdd-eval run --playbook <path> 且 playbook 缺失 workflow.jobs
    当 命令执行
    而且 那么非零退出

  @req:r1
  场景: 空 job steps 被拒
    假如 某 job 定义 steps: []
    当 playbook 校验
    而且 那么校验失败

  @req:r1
  场景: 未知 job 键被拒
    假如 某 job 定义了未知键（如 timeout:）
    当 playbook 校验
    而且 那么校验失败

  @req:r1
  场景: matrix 引用未知 variant 被拒
    假如 某 job 定义 strategy.matrix.variant: ["a"] 但 a 不存在
    当 命令执行
    而且 那么非零退出

  @req:r1
  场景: variant id 不安全用于路径时被拒
    假如 用户定义的 variant id 含 / 或 ..
    当 playbook 校验
    而且 那么校验失败

  @req:r1
  场景: matrix 展开按声明顺序串行执行
    假如 某 job 定义 strategy.matrix.variant: ["b", "a"]
    当 runner 执行
    而且 那么执行该 job 两次
    而且 而且按声明顺序串行

  @req:r1
  场景: needs 引用未知 job 被拒
    假如 某 job 声明 needs: ["missing"]
    当 playbook 校验
    而且 那么校验失败

  @req:r1
  场景: 依赖环被拒
    假如 workflow 含依赖环（直接或间接）
    当 playbook 校验
    而且 那么校验失败
