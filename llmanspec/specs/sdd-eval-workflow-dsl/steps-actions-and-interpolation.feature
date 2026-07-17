# language: zh-CN
# 对应 spec: sdd-eval-workflow-dsl — step kind 为 uses 或 run（互斥）；内置 action 有稳定 id 与
# 沙箱语义；支持最小 ${{ path }} 字符串插值；run 步骤仅允许 allowlist 命令且 cwd 不可穿越沙箱。
功能: step 类型、内置 action、插值与 run 沙箱
  @req:r62
  场景: step 同时含 uses 与 run 被拒
    假如 某 step 同时定义 uses 与 run
    当 playbook 校验
    而且 那么校验失败

  @req:r62
  场景: run step 含 with 被拒
    假如 某 step 定义 run: "rg foo" 且同时含 with: {}
    当 playbook 校验
    而且 那么校验失败

  @req:r62
  场景: 未知内置 action 显式失败
    假如 某 step 使用 builtin:sdd-eval/does-not-exist
    当 命令执行
    而且 那么非零退出

  @req:r62
  场景: workspace.prepare 跳过疑似机密文件
    假如 项目根含 .env 或 .netrc
    当 执行 builtin:sdd-eval/workspace.prepare
    而且 那么这些文件不出现在 variant workspace

  @req:r62
  场景: acp-loop 写出预期工件
    假如 workflow 对某 variant 运行 builtin:sdd-eval/acp.sdd-loop
    当 执行完成
    而且 那么variant 的 logs/ 下存在 acp-session.jsonl

  @req:r62
  场景: 插值在 run 步骤中替换 matrix variant
    假如 matrix 展开的 job step 使用 run: "echo ${{ matrix.variant }}"
    当 runner 执行
    而且 那么先插值再执行

  @req:r62
  场景: 未知插值路径显式失败
    假如 某 step 字符串含 ${{ does.not.exist }}
    当 命令执行
    而且 那么非零退出

  @req:r62
  场景: 非允许命令被拒
    假如 run 步骤请求执行非 allowlist 命令（如 curl）
    当 runner 执行
    而且 那么非零退出

  @req:r62
  场景: cwd 路径穿越被拒
    假如 run 步骤设 cwd: "../outside"
    当 runner 执行
    而且 那么非零退出

  @req:r62
  场景: legacy version:1 playbook 失败并给出可操作错误
    假如 用户运行 llman x sdd-eval run 且 playbook 含 version: 1
    当 命令执行
    而且 那么非零退出
    而且 而且提示 playbook 格式已被 workflow/jobs/steps DSL 取代
