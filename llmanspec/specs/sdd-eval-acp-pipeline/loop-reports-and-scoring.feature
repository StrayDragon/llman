# language: zh-CN
# 对应 spec: sdd-eval-acp-pipeline — SDD loop 经 max_iterations 限界（默认 6）且可复现；
# report 含可对比客观指标；支持 human scoring 导入导出；AI judge 评分可选（需 OPENAI_*）。
功能: 迭代限界、报告生成与可选评分
  @req:r59
  场景: loop 达 max iterations 后停止
    假如 max iterations 设为 3
    当 runner 执行
    而且 那么最多执行 3 次迭代
    而且 而且随后标记该 variant 为 completed-by-limit

  @req:r59
  场景: run 后生成报告
    假如 用户运行 llman x sdd-eval report --run <run_id>
    当 命令执行
    而且 那么在 run 目录下写出报告文件

  @req:r59
  场景: human 评分可导入
    假如 用户运行 llman x sdd-eval import-human --run <run_id> --file scores.json
    当 命令执行
    而且 那么该 run 的报告数据更新为含导入评分

  @req:r59
  场景: 仅启用 AI judge 时缺失 OPENAI key 才失败
    假如 OPENAI_API_KEY 为空且 AI judge 已启用
    当 用户运行 llman x sdd-eval report
    而且 那么非零退出并解释 AI judge 需 OPENAI_API_KEY

  @req:r59
  场景: matrix 引用未知 variant 显式失败
    假如 某 job 定义 strategy.matrix.variant: ["a"] 但 a 不存在
    当 命令执行
    而且 那么非零退出
