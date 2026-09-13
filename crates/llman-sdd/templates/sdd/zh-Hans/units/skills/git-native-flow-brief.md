## Git-native 生命周期（摘要）

勿混淆：**Skill 导航** ≠ **Git-native 生命周期**。全图见根 `AGENTS.md`「领域概念区分」或 `llman-sdd-propose` 内嵌全图。

硬规则：
1. **先** Branch binding（`change start` / `attach`）→ Full；**再** Specs landing（绑定分支编辑并 commit `llmanspec/specs/**`）。
2. 无 live 合约变更 → `needs_specs_change: false`。apply 前须 `readyToImplement=true`。
3. 收口用 `change finalize`（自动提交 `archive(sdd): <id>`；`--no-commit` 可跳过）。`change checkpoint` 已移除（调用即以非零退出报错，指向 finalize）。
4. **禁止**在默认分支 commit live specs；已 attach 勿重复 `start`。
