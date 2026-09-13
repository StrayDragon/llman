# Tasks: rework-finalize-merge-semantics

垂直切片：每个 task 打穿 实现 → 测试 → 门禁 一条窄路径。Seam 确认：CLI 子进程（`llman sdd change start/attach/finalize/archive`，复用既有 BDD step fixture）+ change 模块公共函数（单测，tempfile git 仓）。

- [x] task-1: binding 补全 base_branch
  - `git_native.rs`：`ChangeGitBinding` 增加 `base_branch`；`read_binding` 缺键回退空串（旧 proposal 零迁移）；`write_binding` 三键 upsert。
  - `run_start`/`run_start_worktree`：`base_branch` 恒写默认分支名；`run_attach` 缺省默认分支 + 新增 `--base <branch>` 覆盖（校验该分支引用存在）；`--force` 重绑重算。
  - 单测：start/attach/worktree 三入口写入值、缺键回退、--base 存在性校验、--force 重算。

- [x] task-2: 合并执行器 v2（目标 × 方式）[blocked-by: task-1]
  - `archive.rs`：`do_ff_merge` → `do_merge(root, feature, target, method)`；目标解析 `--into` > binding.base_branch > 默认分支；squash 路径（checkout → merge --squash → rename → 单 commit）与 ff 路径共存；复用 stash 机制。
  - `finalize.rs`/CLI：`finalize`/`archive` 新增 `--into <branch>` 与 `--method <squash|ff>`；flag > config `sdd.merge_method`（FlowConfig 新字段 + schemars 描述 + schema 产物再生）> 缺省 squash。
  - locales（双拷贝同步）：合并结果行、降级 WARNING、手动指引文案。
  - 单测：解析优先级三档、squash 单 commit 形态（目标分支 commit 数 +1 且 message 匹配）、ff 老语义保留、零提交 change、重复合并幂等。

- [x] task-3: worktree 拓扑守卫 + 显式降级 [blocked-by: task-2]
  - `do_merge` checkout 前 `git worktree list --porcelain` 检测目标分支占用：他树持有时跳过合并，输出 WARNING + 可执行手动命令（squash/ff 各自形式），rename 与收尾照常。
  - 单测：主仓持有目标分支时（模拟 linked worktree 场景）finalize 退出为零、归档完成、stdout 含手动指引；占用分支 == 当前分支（即本 worktree）不触发守卫。

- [x] task-4: 文档 / 模板 / locale / schema / 测试适配 [blocked-by: task-2, task-3]
  - `crates/llman-sdd/templates/sdd/**`：llman-sdd-archive/propose/apply-cycle 技能文本去 ff-merge 硬编码（对齐 squash 缺省 + 目标解析），`sdd-structured-skill-prompts` r98 对应措辞已在 Specs landing 落定。
  - 根 `AGENTS.md` 生命周期表（Branch binding 行补 base_branch；收口行补 squash 缺省）+ `llmanspec/AGENTS.md` frontmatter SSOT 表（`base_branch` 行）。
  - `tests/it/sdd_bdd_compat.rs`：finalize 语义断言适配（squash 单 commit、目标分支停留）；BDD 场景 step 词汇核对（不新增 step 词汇，除非复用既有）。
  - `just check-sdd-templates`、`just check-i18n-keys`、`just readme`（CLI 面变化）、schema 产物 `just check-schemas`。

- [x] task-5: 全绿门禁 + E2E [blocked-by: task-4]
  - `just check-all`；`validate --all --strict`；`cargo test --features bdd`。
  - E2E 回归（/tmp 最小复现仓，对齐 issue #19 场景）：linked worktree 中 attach → verify → finalize：退出零、归档完成、目标分支未收口时 WARNING 可见且给出手动命令；主仓场景 squash 后目标分支单 commit。
