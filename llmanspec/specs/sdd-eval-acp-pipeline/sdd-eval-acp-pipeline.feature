# language: zh-CN
# managed by llman sdd partition-migrate
功能: sdd-eval-acp-pipeline

  @req:r1
  场景: help 可用
    假如 用户运行 llman x sdd-eval --help
    当 命令执行
    那么 打印帮助文本并成功退出

  @req:r1
  场景: init 写出 YAML 模板 playbook
    假如 用户在项目根运行 llman x sdd-eval init --name demo
    当 命令执行
    那么 存在 <project>/.llman/sdd-eval/playbooks/demo.yaml
    而且 为可解析 YAML

  @req:r1
  场景: run 创建新 run 目录与基础布局
    假如 用户运行 llman x sdd-eval run --playbook <path>
    当 命令执行
    那么 在 <project>/.llman/sdd-eval/runs/ 下创建新 <run_id> 目录

  @req:r1
  场景: 缺失 variants 显式失败
    假如 playbook 无 variants 且用户运行 llman x sdd-eval run
    当 命令执行
    那么 非零退出并解释至少需要一个 variant

  @req:r1
  场景: variant workspace 用新风格模板初始化
    假如 某 variant workspace 已为某次 run 准备好
    当 初始化执行
    那么 variant workspace 用新风格 SDD 模板初始化

  @req:r1
  场景: run 工件永不含 API key 明文
    假如 用户用含 API key env 的 preset 运行 llman x sdd-eval run
    当 命令执行
    那么 run 目录下无文件含原始 API key 值

  @req:r1
  场景: 路径穿越被拒
    假如 agent 请求读取 ../../.ssh/id_rsa
    当 客户端处理
    那么 拒绝该请求
    而且 在 variant log 记录非机密错误

  @req:r1
  场景: loop 达 max iterations 后停止
    假如 max iterations 设为 3
    当 runner 执行
    那么 最多执行 3 次迭代
    而且 随后标记该 variant 为 completed-by-limit

  @req:r1
  场景: run 后生成报告
    假如 用户运行 llman x sdd-eval report --run <run_id>
    当 命令执行
    那么 在 run 目录下写出报告文件

  @req:r1
  场景: human 评分可导入
    假如 用户运行 llman x sdd-eval import-human --run <run_id> --file scores.json
    当 命令执行
    那么 该 run 的报告数据更新为含导入评分

  @req:r1
  场景: 仅启用 AI judge 时缺失 OPENAI key 才失败
    假如 OPENAI_API_KEY 为空且 AI judge 已启用
    当 用户运行 llman x sdd-eval report
    那么 非零退出并解释 AI judge 需 OPENAI_API_KEY

  @req:r1
  场景: matrix 引用未知 variant 显式失败
    假如 某 job 定义 strategy.matrix.variant: ["a"] 但 a 不存在
    当 命令执行
    那么 非零退出
