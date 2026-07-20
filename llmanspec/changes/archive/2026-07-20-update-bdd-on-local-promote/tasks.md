# Tasks

## P0 — 模板/技能叙事统一

- [x] 1. `templates/sdd/zh-Hans/skills/llman-sdd-archive.md`：`description` 去掉「PR/」；正文 4 处「Git/PR merge / 打开/合并 feature 分支 PR」统一为「本地 `git merge --ff-only` 进默认分支；push/PR 可选」；加「Agent MUST NOT 因本 skill 默认执行 push 或创建 PR」硬约束。
- [x] 2. `templates/sdd/en/skills/llman-sdd-archive.md`：同 1 的英文对等改动。
- [x] 3. `templates/sdd/zh-Hans/skills/llman-sdd-sync.md`：「Git/PR merge」→ 中性「merge 进默认分支」。
- [x] 4. `templates/sdd/en/skills/llman-sdd-sync.md`：同 3 的英文对等改动。
- [x] 5. `templates/sdd/zh-Hans/units/spec/toon-contract.md`：「Git/PR merge promotes specs」→ 中性「merge promotes specs」+ 收尾叙事说明本地 merge 为默认。
- [x] 6. `templates/sdd/en/units/spec/toon-contract.md`：同 5 的英文对等改动。
- [x] 7. `templates/sdd/zh-Hans/skills/llman-sdd-apply-cycle.md`：在「4) 提交」后加「5) 本地合回默认分支」步骤（`git switch <default> && git merge --ff-only <feature>`，可选 `git branch -d <feature>`）；硬约束加「未获用户明确要求时禁止 `git push` / `gh pr create|merge`」。
- [x] 8. `templates/sdd/en/skills/llman-sdd-apply-cycle.md`：同 7 的英文对等改动。
- [x] 9. `AGENTS.md`：BDD-on 段「正常 Git/PR 合并」→「正常 Git 合并（本地 merge 即可；push/PR 可选）」。
- [x] 10. `docs/sdd/pipeline-bdd-on.md`：流程图节点 `Git/PR merge feature → 默认分支` 与 `PR merge` → 统一为本地 merge 语义（如 `merge feature → 默认分支 (local)`）。

## P0 — 渲染副本同步

- [x] 11. 运行 `LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo +nightly run -- sdd init --update` 重新生成 `.agents/skills/**` 渲染副本。
- [x] 12. `git diff .agents/skills/` 复核渲染产物无残留 `{%` / `{{ }}`，且新措辞已正确注入。

## P1 — CLI 用户可见输出

- [x] 13. `locales/app.yml`：新增 `sdd.change.finalize_next_step`（或类似 key），英文文案含「本地 merge 进默认分支；push / hosting PR optional」；`sdd.validate.change_step_1_bdd_on`（BDD-on 专用 next-steps，指向 live specs + attach/finalize）。
- [x] 14. `src/sdd/change/finalize.rs`：成功 `println!` 后追加一行 next-step 提示，走 `t!` 本地化键（与 archive 一致），含本地 merge 语义。
- [x] 15. `src/sdd/shared/validate.rs::print_next_steps`：按 BDD 模式分支 `ItemType::Change`——BDD-on 下打印新 `change_step_1_bdd_on`（指向 live specs + attach/finalize），BDD-off 下保留 `change_step_1`。
- [x] 16. 既有 `sdd-bdd-mode-compat` 相关 `.feature` / `tests/sdd_bdd_compat_tests.rs`：若 finalize stdout / validate next-steps 有 smoke 断言受影响，同步适配（按 AGENTS.md「BDD 模式兼容性测试维护规则」）。

## P2 — 轻量 draft 提案路径 + change id 自动推导

- [x] 17. `src/sdd/change/new.rs`：新增 `--from <description>` flag（或等价）；提供时 `<CHANGE>` 位置参数可选。CLI 从描述生成合法 id（过 `validate_sdd_id`），遵循 `llmanspec/AGENTS.md` 命名约定（无则按语义合理命名）；stdout 打印最终 id + proposal 路径。冲突既有 change 时非零退出并提示 `--force`/换描述。
- [x] 18. `src/sdd/change/new.rs` 的 id 生成逻辑：提取为可测函数（如 `fn derive_change_id(desc: &str) -> Result<String>`），加单元测试覆盖：中文/英文描述、空描述拒绝、非法字符清洗、长度上限、`validate_sdd_id` 通过。
- [x] 19. `src/sdd/command.rs`（或 clap 定义处）：`change new` 的 `<CHANGE>` 改为 `Option`，与 `--from` 互斥校验（两者皆无或皆有时报错）。
- [x] 20. `templates/sdd/zh-Hans/skills/llman-sdd-propose.md`：新增「轻量 draft 路径」节——用户说「draft 提案/change」「记一个提案」且未给 id 时：MUST NOT 询问 id；MUST 用 `change new --from <描述>`（或 skill 内推导）生成 id 并建 proposal.md；告知用户已生成 id（可修改）。硬约束第 1 条「必须确认 id」补充例外：轻量 draft 路径除外。
- [x] 21. `templates/sdd/en/skills/llman-sdd-propose.md`：同 20 的英文对等改动。
- [x] 22. `locales/app.yml`：新增 `change new --from` 相关的成功/错误消息 key（如 `sdd.change.new.derived_id`）。

## P1/P2 — 可执行测试（apply 阶段评估 fixture 可行性后决定）

- [x] 23. 评估 finalize 成功路径 fixture 成本（需真分支 + attach + base_sha）；若可行，为 `local-promote-hints.feature` 第二个场景加 Given step + `#[scenario]` 绑定并标 `@executable`；否则保留为文档型（fast mode）。
- [x] 24. 评估 validate BDD-on change 失败 fixture（需写一个格式不完整的 change）；若可行，加 Given step + `#[scenario]` 绑定并标 `@executable`；否则保留为文档型。
- [x] 25. `change new --from` 的 `@executable` 场景：在 `local-promote-hints.feature`（或新建 `lightweight-draft.feature`）加场景「用户提供描述时 change new 生成合法 id」；加 `#[scenario]` 绑定；断言 stdout 含生成的 id 与 proposal 路径。

## 校验

- [x] 26. `LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo +nightly run -- sdd validate update-bdd-on-local-promote --strict --no-interactive` 通过（fast mode）。
- [x] 27. `LLMAN_CONFIG_DIR=./artifacts/testing_config_home cargo +nightly run -- sdd validate --specs --strict --no-interactive` 通过（fast mode，全局 req_id 唯一性）。
- [x] 28. `just check-sdd-templates` 通过（中英 locale 文件集合对齐 + frontmatter 合规）。
- [x] 29. `just fmt && just lint` 通过。
- [x] 30. `just test`（或 `cargo +nightly test --features bdd`）通过；新增 `#[scenario]` 绑定（若有）的 scenario name 与 `.feature` 字节级匹配。
