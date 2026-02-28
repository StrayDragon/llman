## 背景

`llman sdd` 的 prompt/skills 主要由模板树定义：

- new（默认）：`templates/sdd/{en,zh-Hans}/`
- legacy（显式选择）：`templates/sdd-legacy/{en,zh-Hans}/`

它们驱动两类用户可见输出：

1) 通过 `llman sdd update-skills` 生成的 SDD skills（`llman-sdd-*`，面向 Codex/Claude Code 的 skills 目录）。
2) 通过 `llman sdd update-skills` 生成的 Claude Code workflow commands（`.claude/commands/llman-sdd/*.md`）。

当前主要问题：

- 重复与漂移：guardrails/协议块重复拷贝，导致维护成本高且易失一致。
- 占位/低信号块：例如 “Options / <option …>”，增加噪音并诱导不稳定行为。
- STOP/澄清/校验不统一：不同命令对歧义、缺文件、证据不足、验证失败的处理口径不一致。
- 迭代耦合：模板优化常被 Rust 生成器/测试细节阻塞，不利于“prompt/context 工程师”单独贡献。
- 评估欠标准化：虽然已有 `llman x arena`，但缺少面向 SDD prompts 的“可复现评估套件/一键流程”。

我们希望明确两条互补的贡献者路径，并尽量解耦：

- **Prompt/Context 工程师路径**：只关注 `templates/**` 文本直觉与结构，能用固定命令快速跑门禁与 Arena 验证。
- **项目开发者路径**：在必要时修改 Rust/脚本/测试，为 prompt 工程师提供低摩擦、可回归的迭代环境与发布保障。

## 目标 / 非目标

**Goals:**
- 让 SDD prompts 的编辑更“文本优先 / 直觉优先”：
  - 在合适处将重复片段收敛为 units（或合并为单一真源）
  - 删除占位/低信号块，减少重复 guardrails
  - 统一 STOP/澄清/验证策略，使其可执行且一致
- 在同一变更内同步处理 new + legacy 双轨，并给出“同步/分歧显式化”的规则。
- 提供可复现的评估闭环：
  - 快速门禁：`llman sdd validate --ab-report`（new vs legacy 的结构/治理字段快速对比）
  - 深度评估：Arena（baseline vs candidate），并隔离 `LLMAN_CONFIG_DIR`，避免触碰真实用户配置
- 保持 locale parity（`en` + `zh-Hans`）与模板版本号门禁（`just check-sdd-templates`）。

**Non-Goals:**
- 不引入与 prompt 迭代无关的 SDD “业务功能”变更。
- 不大改 Arena 引擎；评估以既有 `llman x arena` 子命令为基础。
- 不移除 legacy 轨道；必须保持可显式选择与可用。

## 关键决策

### 1) 按变更面解耦职责（Prompt 工程师 vs 项目开发者）

- Prompt/Context 工程师主要修改：
  - `templates/sdd/**` 与 `templates/sdd-legacy/**`
  - 共享 units：`templates/**/units/**`
  - 轻量文档（如何跑门禁 + Arena）
- 项目开发者主要修改：
  - `src/sdd/project/**`：仅当需要新增/调整渲染出口以降低 prompt 工程师摩擦（例如“直接渲染某个 skill 模板用于 Arena”）
  - `tests/sdd_integration_tests.rs` 与检查脚本：锁定关键不变式（治理字段、占位块消失、locale parity 等）

理由：让 prompt 迭代尽量不被生成器/测试细节阻塞；但当结构变化需要工具支持时，仍可由开发者补齐并回归。

### 2) 统一提示结构与“stop-first”策略（降低跑偏）

对每个 workflow 入口（apply/new/continue/ff/verify/explore/sync/archive/show/validate/specs-compact）：

- 明确且一致地写出：输入、前置条件、STOP 条件、验证/校验步骤。
- 删除占位块（如 “Options”），避免在正文之后再次重复一整段 guardrails。
- 结构化协议与治理字段通过 unit 注入保持单一真源；模板不手工复制协议块。

理由：让助手跨入口行为可预测；遇到歧义/证据不足/缺文件时优先停下并询问，避免强行继续。

### 3) new + legacy 同步维护（尽量减少漂移）

legacy 仍被视为兼容面（可显式选择），但本变更选择“同批优化”，以降低长期分叉维护成本。

实现策略：
- 对 new/legacy 采用同一套去冗余与结构化策略，优先让两者语义对齐。
- 如必须分歧（例如 legacy 需要保留某些旧提示习惯），必须在模板头注释与 `sdd-legacy-compat` 增量规范中显式记录理由与影响范围。
- 在贡献者文档中加入“同步检查清单”，避免只改 new 忘记改 legacy。

理由：legacy 存在不应等于“放任腐化”；但它必须可选且不被强制套用 new-only 的额外约束。

### 4) 分层评估闭环：快速门禁 + Arena 深评估

我们将评估分成两层：

1) **快速门禁（无 API Key 也可跑）**
   - `llman sdd validate --ab-report --json --no-interactive`
   - 目的：快速确认 new/legacy 的结构化协议与治理字段不退化（安全/质量优先）。

2) **Arena 深评估（需要模型 API）**
   - 使用隔离的配置目录：`LLMAN_CONFIG_DIR=./artifacts/testing_config_home`（或临时目录），确保不触碰真实用户配置。
   - 如需通过中转/加速访问 OpenAI：设置 `OPENAI_BASE_URL`（优先）或 `OPENAI_API_BASE`（也支持）；若未包含 `/v1` 会自动补齐。
   - 默认评估模型通过 `OPENAI_DEFAULT_MODEL` 指定（例如 `gpt-5.2`）；入口脚本在缺失时应报错或要求显式传参，避免“用错模型”导致结论不可比。
   - 在该目录下维护 repo-tracked 的 Arena dataset/contest 配置（保证可复现）。
   - 提供面向 prompt 工程师的一键流程（文档 + 可选 `just`/脚本），负责：
     1) 从“渲染后的 SDD skills/commands”生成 baseline 与 candidate prompts（例如：legacy vs new，或 before vs after）
     2) 将 prompts upsert 到 `LLMAN_CONFIG_DIR/prompt/<app>/`
     3) 运行 `llman x arena gen` → `vote` → `report`
   - 评分原则：稳定性/安全性（不臆测证据、遇歧义 STOP、验证顺序正确）优先于 token/成本；投票时记录定性失败标签，便于迭代定位。

理由：prompt 改动需要可衡量证据；Arena 提供人类偏好投票 + Elo，并天然支持隔离运行与可重复回放。

### 5) 版本与门禁（避免静默漂移）

- 任意被编辑的模板/unit MUST 同步提升 `llman-template-version`，并在 `en` 与 `zh-Hans` 保持同一路径版本一致（由 `scripts/check-sdd-templates.py` 约束）。
- Arena 运行前 MUST 先跑 `just check-sdd-templates`。
- CI/回归保留：
  - `cargo +nightly test --test sdd_integration_tests -q`（提示词生成不变式）
  - `just test`（当涉及 Rust/脚本改动时的更广回归）

理由：prompt 迭代要快，但不能以静默的 locale 漂移或生成链路破坏为代价。

### 6) `templates/sdd/**/skills/shared.md` 的处置

现状：`templates/sdd/*/skills/shared.md`（以及 legacy 对应文件）并不在 `skill_templates()` 的生成清单内，当前不会影响 `llman sdd update-skills` 的生成结果；它更像历史遗留的“集中说明页”，但内容已可由 `units/` 与各技能模板覆盖，继续保留容易造成“两个真源”的误解。

决策：在本变更中将其视为冗余来源，进行清理：
- 优先方案：删除该文件（new + legacy + 双语同步删除），并在文档中提供等价的“常用命令/协议块”入口（直接指向 `templates/**/units/**`）。
- 如需要保留入口：改为极薄的指针页（不再重复粘贴内容，只链接/引用 units），避免成为影子真源。

## 风险 / 权衡

- [风险] new + legacy 同步维护增加工作量。
  - 缓解：提供同步清单与文档约束；分歧必须显式且尽量罕见。
- [风险] Arena 深评估非确定（模型波动）且依赖 API Key。
  - 缓解：保持数据集小而聚焦；记录定性失败标签；必要时多轮/多 seed。
- [风险] 过度合并/压缩提示词可能丢失有用语义，降低正确性。
  - 缓解：稳定性/安全优先；用集成测试锁定关键不变式；渐进式迭代。
- [风险] prompt-only 改动可能破坏渲染契约（unit id 缺失、模板渲染失败）。
  - 缓解：`check-sdd-templates` + 集成测试 + 渲染缺失单元 fail-fast；必要时提供“直接渲染预览”工具降低试错成本。
