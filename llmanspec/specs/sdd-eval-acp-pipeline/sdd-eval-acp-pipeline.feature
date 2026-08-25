# language: zh-CN
# capability: sdd-eval-acp-pipeline
# purpose: 规范实验性 `llman x sdd-eval` 评测流水线：playbook 驱动、variant 隔离、ACP agent 沙箱与报告生成。
# scope: llmanspec/specs/sdd-eval-acp-pipeline

功能: sdd-eval-acp-pipeline

  @req:r28 @human
  场景: sdd-eval 命令、运行隔离与 ACP 沙箱
    - 对应 spec: sdd-eval-acp-pipeline — CLI MUST 提供 llman x sdd-eval 实验子命令；playbook 置于 .llman/sdd-eval/playbooks/；运行隔离存储于 .llman/sdd-eval/runs/<run_id>/；variants 结合 agent 与 preset；ACP agent 经 preset env 注入启动且不泄漏机密；ACP runner 沙箱限定于 variant workspace。

  @req:r59 @human
  场景: 迭代限界、报告生成与可选评分
    - The system MUST satisfy the harness scenarios for `迭代限界、报告生成与可选评分`: 对应 spec: sdd-eval-acp-pipeline — SDD loop 经 max_iterations 限界（默认 6）且可复现； report 含可对比客观指标；支持 human scoring 导入导出；AI judge 评分可选（需 OPENAI_*）。
