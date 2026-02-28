## 背景与动机

当前 SDD prompt/skills 模板（`templates/sdd/**` 与 `templates/sdd-legacy/**`）总体有效，但已累积出明显的维护与执行风险：

- 文本冗余与漂移：重复的 guardrails/协议块分散在多个模板中，改动容易漏改、难以保持一致。
- 低信号占位块：存在 “Options / <option 1>” 等占位内容，既占 token，又会诱导不稳定输出。
- STOP/澄清/校验口径不一致：不同 workflow 对“何时必须停下并询问/何时必须验证”的表达不统一，增加跑偏与误改风险。

同时，我们需要解耦两类贡献者的关注点：

- **Prompt/Context 工程师**：主要通过编辑模板文本来优化行为，并能用可复现的评估闭环（Arena）验证，不需要理解/修改 Rust 生成器细节。
- **项目开发者**：在必要时改动生成器/测试/门禁，为 prompt 工程师提供更低摩擦的迭代路径，并确保可发布质量。

## 变更内容

- 同步优化 new + legacy 双轨模板：
  - new（默认）：`templates/sdd/**`
  - legacy（显式选择）：`templates/sdd-legacy/**`
- 去繁就简与合并：
  - 删除低信号占位块（例如 “Options / <option …>”）
  - 合并重复段落，统一 STOP/澄清/验证策略的表达
  - 在不削弱治理字段（ethics governance）的前提下，降低提示词熵与维护成本
- 建立“prompt 工程师闭环”：
  - 固化一套可复现的 Arena 评估流程（baseline vs candidate：人工投票 + Elo + 定性失败标签）
  - 提供面向模板编辑者的最小化运行手册与辅助脚本/just 命令（如需要）
- 保持双语一致性与门禁：
  - `en` 与 `zh-Hans` 维持模板集合一致与版本号一致（`just check-sdd-templates`）
  - 关键不变式由集成测试锁定（避免回退）

## 涉及能力项

### 新增能力项
- 无（不新增 capability）

### 调整能力项
- `sdd-structured-skill-prompts`：在保留治理字段与结构化协议的前提下，进一步去冗余、去占位，并将 STOP/澄清/验证策略统一为可执行约束。
- `sdd-template-units-and-jinja`：强化 unit/模板的可发现性与单一事实来源，支持“只改模板也能跑通评估”的低耦合迭代。
- `sdd-ab-evaluation`：将 prompt 评估从“主观感受”升级为可复现的 Arena 闭环（安全/质量优先于 token/延迟），并与内建 A/B 报告一致。
- `sdd-legacy-compat`：明确 legacy 双轨的维护规则（同步更新或显式记录分歧），确保 legacy 可用且不意外漂移。

## 影响范围

- 受影响模板（new + legacy 双轨）：
  - `templates/sdd/{en,zh-Hans}/skills/*.md`
  - `templates/sdd/{en,zh-Hans}/spec-driven/*.md`
  - `templates/sdd-legacy/{en,zh-Hans}/skills/*.md`
  - `templates/sdd-legacy/{en,zh-Hans}/spec-driven/*.md`
  - 共享注入单元：`templates/**/units/**`（必要时新增/合并）
- 受影响门禁/工具：
  - `scripts/check-sdd-templates.py`（如需增加/调整“占位块/重复块”检测）
  - `justfile` / `scripts/`（如需提供“一键 Arena 评估”）
  - `docs/arena.md` 或新增文档（固化 prompt 工程师评估流程）
- 受影响代码（尽量最小，仅用于解耦与可用性增强）：
  - `src/sdd/project/**`（模板加载/渲染/生成路径，如需提供“直接渲染某个 skill 模板用于 Arena”的出口）
  - `src/sdd/shared/validate.rs`（如扩展 `--ab-report` 以复用评估产物/输出）
  - `tests/sdd_integration_tests.rs`（锁定关键不变式：治理字段不缺失、占位块不出现等）
