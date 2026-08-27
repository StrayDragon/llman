# Tasks — feat-sdd-review-workflow-suite

Seam 确认（r101，用户已批）：S1=`llman sdd review` CLI 子进程；S2=`--export-html`
文件产物边界；S4=review↔list/show 数据互恰。T0 为行为冻结地基，不新增 seam。
决议引用：D-A 模板 canonical 在 templates/sdd/shared/ + include_str 编译内嵌；
D-B 零配置 v1；D-C 锁定 diff 仅提示；D-D checkpoint 进 AGENTS.md 自由区。
新 capability 合约在绑定分支落地为 llmanspec/specs/sdd-review/sdd-review.feature
（req 自 r5 分配）；下列切片为其实现侧。完成时按归档惯例逐条改写为行内
`- [x] Tn: … [blocked-by: …]`（避免行首方括号被解析器误判，见已归档教训）。

## T0 presenters-deepen-behavior-frozen
show_spec 拆 `show_spec_json(meta|full)` / `show_spec_text` 小接口；morphology
装配收敛单一 helper；快照单测 + 既有 r134 executable 回归作冻结证明。
依赖：无

## T1 human-review-checkpoint-docs
root AGENTS.md 自由区新增 Human Review Checkpoint 小节（何时审/命令序列/
分歧升级路径）；propose/verify 技能模板补人读摘要段要求，en/zh parity 过
check-sdd-templates。
依赖：无

## T2 review-aggregate-core
新子命令 `sdd review [--capability] [--json]`：聚合 pending/manual rules、
harness unbound、staleness、locked-diff 提示（D-C 仅提示）、validate FAIL/WARNING；
存在 CRITICAL 即非零退出；--json 与文本同构。S1+S4 executable 场景挂 r5-r7，
数字与 list --specs 形态互恰（r3/r39 口径）。
依赖：T0

## T3 export-html-single-file
templates/sdd/shared/review.html 共享模板 + include_str 编译内嵌；
mermaid capability↔req↔scenario 层级图 + 过滤器；动态文本最小转义单测。
S2 executable 场景挂 r8（相对路径存在 + 内容包含 mermaid 标记）。
依赖：T2

## T4 docs-gates-sweep
cli help / locales 键补全；check-sdd-templates.py 对 shared/ 增轻校验白名单
（文件存在 + 含 html 收尾，不动 parity 机制）；全量门禁收口：
validate --all --strict（含 full mode）、bdd runner、clippy -D warnings。
依赖：T1, T2, T3
