# sdd-ab-evaluation — Delta Specification（sdd-style-eval-v3）

## ADDED Requirements

### Requirement: 更贴近真实的 spec-only 小项目（三次 change 循环）
评测套件 MUST 包含一个 spec-only（只写规范，不写代码）的小项目场景（TODO app），在同一 workspace 中连续执行三次 change 循环，使 agent 必须反复构建上下文并遵循 SDD 工作流。

#### Scenario: Three cycles in one workspace
- **WHEN** v3 multi-style agentic eval 以 `spec_style: ison|toon|yaml` 运行
- **THEN** agent 在同一 workspace 中完成且只完成三次 `propose -> apply -> archive -> commit` 迭代
- **AND** 每次迭代对应 `llmanspec/changes/**` 下的一个独立 change
- **AND** workspace 在 strict mode 下保持有效（`llman sdd validate --all --strict --no-interactive` 通过）

### Requirement: apply 阶段允许 CLI 写入，但仍必须包含文件级 spec 编辑
v3 场景 MUST 允许在 apply 阶段使用 `llman sdd spec add-*` / `llman sdd delta add-*` 等命令，但 MUST 仍要求至少一次对 style-specific 的 `spec.md` 做文件级编辑，从而让格式差异进入 agent 的上下文与操作路径。

#### Scenario: CLI may be used, but file edits are mandatory
- **WHEN** agent 在 apply 阶段实施一个 change
- **THEN** 它 MAY 使用 CLI helper（例如 `llman sdd spec add-*` / `llman sdd delta add-*`）
- **AND** 它 MUST 在读取后，直接编辑至少一个 `llmanspec/**/spec.md` 文件（file-level edit）
- **AND** 被编辑文件对该 workspace 仍保持 style-correct（fence 与 payload 正确）

### Requirement: runner 预置的 TODO skeleton 与三套 change skeleton 在各 style 下语义等价
eval runner MUST 在每个 style workspace 中预先 seed 语义等价的 TODO app baseline 与三套 change skeleton（包含确定性的 markers），以降低方差并避免 agent 在不同 style 下漂移成不同的项目形态。

#### Scenario: Runner seeds v3 skeleton before evaluation
- **WHEN** 一个新的 v3 eval run 开始
- **THEN** runner 创建三个隔离 workspace（`ison/toon/yaml`）
- **AND** 在每个 workspace 中 seed 等价的 TODO app baseline spec + 三套 change skeleton
- **AND** seed 的内容对该 workspace 保持 style-correct

### Requirement: score-only 的 judge 输出可被采集（Optional）
当启用 rubric judge 评分（例如 `--judge claude`）时，评测输出 MUST 包含 judge 分数，但该分数 MUST NOT 成为主要的 pass/fail gate（strict validate 仍是 gate）。

#### Scenario: Judge enabled produces scores without gating
- **WHEN** 维护者为 agentic eval 启用 rubric scoring
- **THEN** Promptfoo 输出在 `results.json` 中包含 rubric score 字段
- **AND** batch aggregate 报告包含各 style 的分数分布（mean/median/p90）
- **AND** pass/fail 仍由硬门禁断言决定（包含 strict validate）
