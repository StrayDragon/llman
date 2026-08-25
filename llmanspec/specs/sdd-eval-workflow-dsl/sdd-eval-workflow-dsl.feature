# language: zh-CN
# capability: sdd-eval-workflow-dsl
# purpose: 规范 `llman x sdd-eval` playbook 的 workflow/jobs/steps DSL：依赖、matrix 展开、内置 action、插值与 run 沙箱。
# scope: llmanspec/specs/sdd-eval-workflow-dsl

功能: sdd-eval-workflow-dsl

  @req:r29 @human
  场景: playbook 采用 workflow/jobs/steps 模型与确定性依赖
    - 对应 spec: sdd-eval-workflow-dsl — playbook MUST 为 YAML 文档，定义 workflow/jobs/steps 模型； variants 按稳定 id 寻址并可经 matrix 展开；job needs 依赖确定性解析（拓扑序 + 声明序兜底）。

  @req:r62 @human
  场景: step 类型、内置 action、插值与 run 沙箱
    - The system MUST satisfy the harness scenarios for `step 类型、内置 action、插值与 run 沙箱`: 对应 spec: sdd-eval-workflow-dsl — step kind 为 uses 或 run（互斥）；内置 action 有稳定 id 与 沙箱语义；支持最小 ${{ path }} 字符串插值；run 步骤仅允许 allowlist 命令且 cwd 不可穿越沙箱。
