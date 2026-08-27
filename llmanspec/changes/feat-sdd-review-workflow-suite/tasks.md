# Tasks — feat-sdd-review-workflow-suite

Seam 确认（r101，用户已批）：S1=`llman sdd review` CLI 子进程；S2=`--export-html`
文件产物边界；S4=review↔list/show 数据互恰。T0 为行为冻结地基，不新增 seam。
决议引用：D-A 模板 canonical 在 templates/sdd/shared/ + include_str 编译内嵌；
D-B 零配置 v1；D-C 锁定 diff 仅提示；D-D checkpoint 进 AGENTS.md 自由区。
新 capability 合约已落地：llmanspec/specs/sdd-review/sdd-review.feature
（r5/r6/r20/r38/r51；landing commit f672634）。

- [x] T0 presenters-deepen-behavior-frozen: render_spec_json/render_spec_text 纯函数拆分；t0_freeze_tests 三例冻结 meta/full 键集与文本形态 [blocked-by: none]
- [x] T1 human-review-checkpoint-docs: AGENTS.md 自由区 Human Review Checkpoint 小节；双语新 unit human-readable-summary 并接入 propose/verify 四模板（注册进 UNIT_FILES+include map）；init --update 重渲染；check-sdd-templates 绿 [blocked-by: none]
- [x] T2 review-aggregate-core: src/sdd/review.rs 五信号聚合（复用 morphology/staleness/lock_gate/validate 子进程同源取数）；--capability/--json/退出码策略；S1+S4 executable 挂 r5/r20/r38 全过 [blocked-by: T0]
- [x] T3 export-html-single-file: templates/sdd/shared/review.html（零外部资源、textContent 注入防逃逸）+ include_str 内嵌 + mermaid 层级图；S2 executable 挂 r51 过 [blocked-by: T2]
- [x] T4 docs-gates-sweep: cli help/枚举接线；check-sdd-templates.py locale 发现白名单（skills/ 子目录判定）+ shared/*.html 轻校验；新增泛型步骤 3 个（损坏 proposal Given、JSON 键存在、点路径数字断言）。全量门禁：fmt/clippy -D 0、非 BDD 26 target ok、BDD runner 56/56、validate --all --strict 34/34、check-sdd-templates 绿 [blocked-by: T1, T2, T3]

自修复轮次记录：
- Round 1：review critical 虚高 — lock_gate 返回的 INFO 级条目被计入 critical；改为仅 ValidationLevel::Error 计数 → 绿
- Round 2：bdd 全量 49 失败 — 新 unit human-readable-summary 未注册进 templates.rs 的 UNIT_FILES 与 include_str 映射，init 渲染即断；补注册 → 绿
- Round 3：review-json-shape 步骤缺裸变体绑定 + criticalCount 嵌套于 summary — 补两个泛型 Then（键存在 / 点路径数字断言），feature 行改为 summary.criticalCount → 56/56
