<!-- llman-template-version: 1 -->
<!-- source: OpenSpec src/core/templates/skill-templates.ts:getOnboardInstructions (copied 2026-02-09; adapted for llman) -->

带用户完成第一次 llman SDD 的完整闭环（从想法到实现再到归档）。这是一个教学体验：你会在真实代码库里做真实工作，同时解释每一步的目的。

---

## 预检查（Preflight）

确认仓库已经初始化为 llman SDD：

- 如果仓库根目录没有 `llmanspec/`，让用户先运行：
  ```bash
  llman sdd init
  ```
  然后在 `llmanspec/` 存在后继续。

---

## 阶段 1：欢迎

展示：

```
## Welcome to llman SDD!

我们将用你代码库中的一个真实小任务，走完一次完整的 change cycle（从想法 → 规范 → 实现 → 归档）。

**我们会做什么：**
1. 选一个足够小的真实任务
2. 简短探索问题
3. 创建 change（承载容器）
4. 生成 artifacts：proposal → specs → design（可选）→ tasks
5. 按 tasks 实现代码
6. 校验并归档变更

**时间：** ~15-30 分钟（取决于任务规模）
```

暂停并询问：
> "准备好选一个小的 starter task 了吗？"

---

## 阶段 2：选择任务

扫描代码库，找 3-4 个具体的小改进点，并给出可选列表。

常见线索：
- `TODO` / `FIXME` / `HACK`
- 缺少校验或错误处理
- 小范围重构以提高可读性
- 小的纯函数缺测试
- 提交里残留的 debug 语句

如果没有明显 quick win，直接问用户想修哪个小问题或加哪个小功能。

范围护栏：
- 如果任务太大，建议切一个更小的 slice，以便走完整个流程。

---

## 阶段 3：创建 Change（Artifacts）

1. 选择一个 kebab-case 的 change id，并带动词前缀（`add-`、`update-`、`remove-`、`refactor-`）。
2. 按顺序在 `llmanspec/changes/<id>/` 下创建 artifacts：
   - `proposal.md`（why/what/impact）
   - `specs/<capability>/spec.md`（delta requirements + scenarios）
   - `design.md`（仅当需要讨论权衡/架构时）
   - `tasks.md`（有序、小步、可验证）
3. 校验：
   ```bash
   llman sdd validate <id> --strict --no-interactive
   ```

在关键节点（proposal 完成、tasks 完成）暂停，向用户确认后再继续。

---

## 阶段 4：实施

1. 阅读你刚创建的 artifacts。
2. 按顺序实现 `tasks.md`。
3. 每完成一项任务，立即更新 checkbox（`- [ ]` → `- [x]`）。
4. 遇到歧义/阻塞时暂停并询问用户下一步。

实现后再次校验：
```bash
llman sdd validate <id> --strict --no-interactive
```

---

## 阶段 5：归档

当变更已被接受/部署后：

```bash
llman sdd archive <id>
```

然后运行：
```bash
llman sdd validate --strict --no-interactive
```

---

## 护栏

- starter task 必须足够小，确保能走完闭环
- 讲清楚关键决策即可，避免长篇说教
- 不要跳过校验步骤
- 改动保持最小，严格围绕 tasks
