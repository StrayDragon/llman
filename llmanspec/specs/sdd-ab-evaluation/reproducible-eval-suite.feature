# language: zh-CN
# 对应 spec: sdd-ab-evaluation — SDD workflow MUST 提供可复现的 Promptfoo 评测套件对比不同风格/
# 版本 SDD prompt；资产存放于 agentdev/promptfoo/；评测在隔离临时目录运行；Claude Code agentic
# 评测可用 Promptfoo 自动驱动并用硬门禁判定通过。
功能: 可复现的 SDD prompts 评测套件与 Claude Code agentic 评测
  场景: 在共享场景集上运行对比评测
    假如 用户对目标场景集执行评测流程
    当 评测运行
    那么系统在等价输入上运行 legacy 与新风格生成/评测

  场景: 评测在隔离配置目录下运行
    假如 维护者运行评测脚本并从 agentdev/promptfoo/ 读取 fixtures 与模型列表
    当 评测运行
    那么Promptfoo 数据写入该流程创建的临时工作目录

  场景: 评测套件位于 agentdev
    假如 维护者在仓库查找 promptfoo 评测套件位置
    当 查找
    那么评测套件位于 agentdev/promptfoo/

  场景: 运行一次 Claude Code agentic 评测
    假如 维护者运行 Promptfoo fixture 驱动 Claude Code agent 完成一轮 SDD authoring + validate
    当 评测运行
    那么输出含 results.json 与 results.html
