# language: zh-CN
# managed by llman sdd partition-migrate
功能: sdd-eval-workflow-dsl

  @req:r1
  场景: 缺失 workflow.jobs 时显式失败
    假如 用户运行 llman x sdd-eval run --playbook <path> 且 playbook 缺失 workflow.jobs
    当 命令执行
    那么 非零退出

  @req:r1
  场景: 空 job steps 被拒
    假如 某 job 定义 steps: []
    当 playbook 校验
    那么 校验失败

  @req:r1
  场景: 未知 job 键被拒
    假如 某 job 定义了未知键（如 timeout:）
    当 playbook 校验
    那么 校验失败

  @req:r1
  场景: matrix 引用未知 variant 被拒
    假如 某 job 定义 strategy.matrix.variant: ["a"] 但 a 不存在
    当 命令执行
    那么 非零退出

  @req:r1
  场景: variant id 不安全用于路径时被拒
    假如 用户定义的 variant id 含 / 或 ..
    当 playbook 校验
    那么 校验失败

  @req:r1
  场景: matrix 展开按声明顺序串行执行
    假如 某 job 定义 strategy.matrix.variant: ["b", "a"]
    当 runner 执行
    那么 执行该 job 两次
    而且 按声明顺序串行

  @req:r1
  场景: needs 引用未知 job 被拒
    假如 某 job 声明 needs: ["missing"]
    当 playbook 校验
    那么 校验失败

  @req:r1
  场景: 依赖环被拒
    假如 workflow 含依赖环（直接或间接）
    当 playbook 校验
    那么 校验失败

  @req:r1
  场景: step 同时含 uses 与 run 被拒
    假如 某 step 同时定义 uses 与 run
    当 playbook 校验
    那么 校验失败

  @req:r1
  场景: run step 含 with 被拒
    假如 某 step 定义 run: "rg foo" 且同时含 with: {}
    当 playbook 校验
    那么 校验失败

  @req:r1
  场景: 未知内置 action 显式失败
    假如 某 step 使用 builtin:sdd-eval/does-not-exist
    当 命令执行
    那么 非零退出

  @req:r1
  场景: workspace.prepare 跳过疑似机密文件
    假如 项目根含 .env 或 .netrc
    当 执行 builtin:sdd-eval/workspace.prepare
    那么 这些文件不出现在 variant workspace

  @req:r1
  场景: acp-loop 写出预期工件
    假如 workflow 对某 variant 运行 builtin:sdd-eval/acp.sdd-loop
    当 执行完成
    那么 variant 的 logs/ 下存在 acp-session.jsonl

  @req:r1
  场景: 插值在 run 步骤中替换 matrix variant
    假如 matrix 展开的 job step 使用 run: "echo ${{ matrix.variant }}"
    当 runner 执行
    那么 先插值再执行

  @req:r1
  场景: 未知插值路径显式失败
    假如 某 step 字符串含 ${{ does.not.exist }}
    当 命令执行
    那么 非零退出

  @req:r1
  场景: 非允许命令被拒
    假如 run 步骤请求执行非 allowlist 命令（如 curl）
    当 runner 执行
    那么 非零退出

  @req:r1
  场景: cwd 路径穿越被拒
    假如 run 步骤设 cwd: "../outside"
    当 runner 执行
    那么 非零退出

  @req:r1
  场景: legacy version:1 playbook 失败并给出可操作错误
    假如 用户运行 llman x sdd-eval run 且 playbook 含 version: 1
    当 命令执行
    那么 非零退出
    而且 提示 playbook 格式已被 workflow/jobs/steps DSL 取代
