---
depends_on: []
---

# 实验性 en-SSOT 模板 + per-locale 语言提示尾巴

## Why

- 现状：`templates/sdd/` 维护 en 与 zh-Hans 两棵全量树（各 29 个 md），每次 en 措辞调整都要同步翻译。历史上已真实发生过两类漂移（propose 整篇未翻译、archive 的 toon 阻断规则过期）；`check-sdd-templates.py` 已加正文级门禁拦截，但双树的同步维护成本仍在。
- 调研结论（2026-08-25，5 款分词器实测同一批渲染产物）：zh 相对 en 的 token 溢价在 cl100k 上为 +30%，但在现代中文优化分词器上仅 +4.5% ~ +12%（见 Impact 留档数据）。项目主要面向中文模型用户，token 成本不足以成为独立动机；本需求定位为**可维护性增强**（单 SSOT + 可精细化调校英文措辞 + 跨模型 token 计数可预测），以实验特性验证，默认关闭。
- 语言行为（混搭时以中文为主、概念双语标注）不依赖全量中文模板，一个 per-locale 提示尾巴即可承载。

## What Changes

- 新增实验配置（命名正式化时定夺，示意 `experimental.en_ssot_locale_hint: true`），默认 false。
- 开关开启时：
  - 模板解析只用 `templates/sdd/en/**` 作为 SSOT，跳过其他 locale 全量树；
  - 渲染每个 SKILL.md（及 agents stub）时在正文末尾追加 `templates/sdd/hints/<locale>.md` 语言提示尾巴（zh-Hans 草案：与用户交流/写文档以中文为主；专有概念用「中文（English）」双语标注；代码、命令、路径、Gherkin 关键字保持原样）；
  - Gherkin 语言仍由 config `locale`（`bdd.default_language` 优先）驱动，不受影响。
- 开关关闭（默认）：行为与现状完全一致，zh-Hans 全量树继续生效。
- zh-Hans 全量树的移除推迟到实验转正后另行 change 处理。
- 实现落点（正式化时细化）：渲染钩子在 `update_skills.rs::write_tool_skills` 与 `init.rs` stub 写入；`check-sdd-templates.py` 增加 hints 文件存在性校验。

## Capabilities

- `sdd-template-units-and-jinja`：locale 查找语义扩展出「en SSOT + hint」模式（实验开关内）。
- `cli-experience` / `sdd-bdd-mode-compat`：若 `--lang` 输出语义或渲染 smoke 断言受影响，正式化时一并评估。

## Impact

- 用户可见性：仅显式开启实验开关的项目受影响；默认行为零变化。
- 决策依据数据（留档；测量对象为默认 10 个 skill 的渲染产物，HF 分词器 `add_special_tokens=False`）：

  | 分词器 | en | zh-Hans | zh/en |
  |---|---|---|---|
  | cl100k_base（GPT-3.5/4 遗留） | 22,994 | 29,956 | 1.303 |
  | o200k_base（GPT-4o 一代） | 23,166 | 25,979 | 1.121 |
  | Qwen2.5 | 23,022 | 25,384 | 1.103 |
  | Qwen3 | 23,022 | 25,384 | 1.103 |
  | DeepSeek-V3 | 23,876 | 24,954 | 1.045 |

  未测：Claude（闭源分词器）；GLM-4（仓库 gated 且为自定义词表格式，离线无法精确复现）——以 o200k/Qwen 一档作为代理。en 总数跨分词器稳定（±2%），zh 波动 ±20%。
- 结论要点：方向上 en 恒不劣于 zh（五款分词器一致），但幅度在中文优化分词器上接近按需加载的噪音级；推动本提案的主因是 SSOT/可维护性而非 token。
- 后续：准备好落实时经 `llman-sdd-propose` 正式化（triage + tasks → Branch binding → Specs landing）。
