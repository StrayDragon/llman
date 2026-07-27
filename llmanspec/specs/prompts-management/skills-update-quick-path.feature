# language: zh-CN
# 对应 spec: prompts-management r82 — init --update 生成 quick/triage/context 引导。
功能: init --update 含 quick 路径与 triage 引导
  @req:r82
  场景: init --update 产出含 quick 与 context/triage 指引
    假如 agent 运行 llman sdd init --update
    当 生成完成
    那么 产物含 llman-sdd-quick
    而且 各 skill 引用 context 命令、异步重建引导与 triage 规则
