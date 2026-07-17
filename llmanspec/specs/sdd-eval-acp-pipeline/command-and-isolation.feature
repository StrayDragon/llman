# language: zh-CN
# 对应 spec: sdd-eval-acp-pipeline — CLI MUST 提供 llman x sdd-eval 实验子命令；playbook 置于
# .llman/sdd-eval/playbooks/；运行隔离存储于 .llman/sdd-eval/runs/<run_id>/；variants 结合 agent
# 与 preset；ACP agent 经 preset env 注入启动且不泄漏机密；ACP runner 沙箱限定于 variant workspace。
功能: sdd-eval 命令、运行隔离与 ACP 沙箱
  @req:r28
  场景: help 可用
    假如 用户运行 llman x sdd-eval --help
    当 命令执行
    而且 那么打印帮助文本并成功退出

  @req:r28
  场景: init 写出 YAML 模板 playbook
    假如 用户在项目根运行 llman x sdd-eval init --name demo
    当 命令执行
    而且 那么存在 <project>/.llman/sdd-eval/playbooks/demo.yaml
    而且 而且为可解析 YAML

  @req:r28
  场景: run 创建新 run 目录与基础布局
    假如 用户运行 llman x sdd-eval run --playbook <path>
    当 命令执行
    而且 那么在 <project>/.llman/sdd-eval/runs/ 下创建新 <run_id> 目录

  @req:r28
  场景: 缺失 variants 显式失败
    假如 playbook 无 variants 且用户运行 llman x sdd-eval run
    当 命令执行
    而且 那么非零退出并解释至少需要一个 variant

  @req:r28
  场景: variant workspace 用新风格模板初始化
    假如 某 variant workspace 已为某次 run 准备好
    当 初始化执行
    而且 那么variant workspace 用新风格 SDD 模板初始化

  @req:r28
  场景: run 工件永不含 API key 明文
    假如 用户用含 API key env 的 preset 运行 llman x sdd-eval run
    当 命令执行
    而且 那么run 目录下无文件含原始 API key 值

  @req:r28
  场景: 路径穿越被拒
    假如 agent 请求读取 ../../.ssh/id_rsa
    当 客户端处理
    而且 那么拒绝该请求
    而且 而且在 variant log 记录非机密错误
