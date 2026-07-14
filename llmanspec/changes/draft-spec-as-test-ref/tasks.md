# Tasks: feature-as-spec (draft-spec-as-test-ref)

## Phase 1: 死代码清理 + IR 调整 (P0)

- [ ] **1. 删除 `feature_refs` IR 字段** — 在 `src/sdd/spec/ir.rs` 删除 `MainSpecDoc.feature_refs` 字段（行 18-20）和 `FeatureRefEntry` 结构体（行 23-29）。
- [ ] **2. 删除 `FeatureRef` presentation wrapper** — 在 `src/sdd/spec/parser.rs`（行 21-36）删除 `FeatureRef` 及其 `From` impl，清理所有引用点。
- [ ] **3. 删除 `validate_feature_refs`** — 在 `src/sdd/spec/validation.rs`（行 614-728）删除函数；保留其 gherkin 解析逻辑供新 validator 复用。
- [ ] **4. 清理调用点** — 从 `validate_spec_content_with_frontmatter_and_bdd`（validation.rs:126）移除 `validate_feature_refs` 调用；更新 `BddConfig` 相关传参。
- [ ] **5. 跑 `just check`** — 确保 fmt/clippy/test 全绿，死代码清理无回归。

## Phase 2: Directory-based feature discovery + fast mode (P0)

- [ ] **6. 实现 `discover_features(spec_dir)`** — 新函数：glob `<spec_dir>/*.feature`，返回有序路径列表。放在 `src/sdd/spec/validation.rs` 或新模块。
- [ ] **7. 实现 `validate_features_dir()`** — 对每个 `.feature`：`gherkin::Feature::parse(content, GherkinEnv::new(lang))`。lang 由 `locale_to_gherkin_lang(config.locale)` 推导。解析失败 → ERROR + 修复提示。
- [ ] **8. 实现 `locale_to_gherkin_lang()`** — 映射 `zh-Hans*` → `zh-CN`，其余透传。优先使用 `BddConfig.default_language`（若设）。
- [ ] **9. 重写 point-only guardrail** — 在 `validate_main_spec_doc`（validation.rs:745-777）：BDD-on + 目录有 features + requirements 空 → OK（Info：feature-as-spec 模式）；BDD-on + 目录无 Features + requirements 空 → ERROR（空 spec）。
- [ ] **10. 接入校验流水线** — `validate_spec_content_with_frontmatter_and_bdd`：`bdd_enabled` 为真时调 `validate_features_dir()` 取代旧 `validate_feature_refs`。
- [ ] **11. 写单元测试** — `validate_features_dir`：合法 feature / 语法错误 / 中文关键字 / 空目录 + 有 requirements / 空目录 + 无 requirements。用 `TempDir` 避免污染。

## Phase 3: BDD-on 门控 + valid_scope 退休 (P0)

- [ ] **12. BDD-on 模式忽略 `valid_scope`** — `validate_spec_meta`：`bdd_enabled` 为真时不要求/不校验 `valid_scope`（不报缺失）。
- [ ] **13. Scope hook 在 BDD-on 退休** — staleness 校验（r15 逻辑）：BDD-on 模式跳过 `valid_scope` 匹配，改为"spec 目录下任意文件变更 = 该 spec 被触及"。
- [ ] **14. 测试门控分支** — BDD-off 全套行为不变回归；BDD-on 新行为覆盖。

## Phase 3.5: archive 扩展——复制 .feature (P0)

- [ ] **14a. 扩展 `find_spec_updates` 收集 .feature** — 在 `src/sdd/change/archive.rs`：BDD-on 模式下，除了收集 `spec.toon`（现有 SpecUpdate），额外收集 `change/specs/<capability>/*.feature` 路径列表。
- [ ] **14b. 实现 .feature 复制逻辑** — 在 spec.toon merge 之后、change 目录 rename 之前，将收集的 `.feature` 复制到主 `specs/<capability>/`。目标同名已存在 → 报错中止（不覆盖），与 r7 冲突策略一致。
- [ ] **14c. BDD-off 模式跳过** — 无 `bdd:` 段时 archive 行为完全不变（不执行 .feature 复制）。
- [ ] **14d. 测试 archive 复制** — 用 `TempDir` 造 change + .feature + 主 spec 目录：正常复制 / 同名冲突报错 / BDD-off 不复制。

## Phase 4: full mode (P1)

- [ ] **15. 加 `--check` flag** — `src/sdd/command.rs` 的 `Validate` 命令新增 `--check: bool`。
- [ ] **16. 实现 full mode 执行** — fast mode 通过后，读 `bdd.effective_run_command()`，shell-out 对整个 spec 目录跑一次（非逐 feature）。退出码 0 → pass；非 0 → fail（输出 runner stderr）。
- [ ] **17. 汇总报告** — full mode 输出：`N features parsed, M passed / K failed`。
- [ ] **18. 测试 full mode** — 用 mock runner / echo 脚本验证退出码映射（`TempDir` + 可执行脚本）。

## Phase 5: Skills 约定更新 (P2)

- [ ] **19. 更新 `llman-sdd-propose` skill** — `templates/sdd/{en,zh-Hans}/skills/llman-sdd-propose.md`：加 BDD-on 分支——创建 `.feature`（按 locale 关键字）+ 从 requirements 删对应 statement。
- [ ] **20. 更新 `llman-sdd-apply` skill** — BDD-on：读 spec 目录 `.feature` → 实现 step definitions → `validate <spec> --check`。
- [ ] **21. 更新 `llman-sdd-verify` skill** — BDD-on：fast (`validate <spec>`) + full (`validate <spec> --check`) 双层。
- [ ] **22. 更新 `llman-sdd-archive` skill** — `.feature` 随 spec 目录一起归档/合并（无需特殊处理，确认即可）。
- [ ] **23. 跑 `just check-sdd-templates`** — 验证模板版本头与 locale 一致性。

## Phase 6: 验证与归档

- [ ] **24. 跑 `just check-all`** — fmt + clippy + test + docs + release build + SDD 模板检查全绿。
- [ ] **25. 跑 `llman sdd validate draft-spec-as-test-ref --strict --no-interactive`** — 本 change 自校验通过。
- [ ] **26. 手动 smoke test** — 在临时目录造一个 BDD-on 项目：config.yaml 加 `bdd:` 段 + spec 目录放中/英 `.feature` + 跑 fast/full mode。
- [ ] **27. 归档** — `llman sdd archive run draft-spec-as-test-ref` + git commit。
