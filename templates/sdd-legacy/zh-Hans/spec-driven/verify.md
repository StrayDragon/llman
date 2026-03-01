<!-- llman-template-version: 2 -->
<!-- source: OpenSpec templates/zh-Hans/llman-sdd/verify.md (copied 2026-02-09) -->

在归档前，验证实现是否与变更工件（specs / tasks / design）一致。

**输入**：可选在 `/llman-sdd:verify` 后指定变更 id（例如 `/llman-sdd:verify add-auth`）。如果省略，先从上下文推断；若不明确，必须让用户选择。

**步骤**

1. **选择变更**

   - 若提供了 id，直接使用。
   - 否则：
     - 若对话上下文明确指向某个变更 id，则使用它。
     - 否则运行 `llman sdd-legacy list --json`，让用户选择要验证的变更。

2. **加载工件**

   读取 `llmanspec/changes/<id>/` 下已存在的内容：
   - `proposal.md`（如果存在）
   - `specs/*/spec.md`（所有 delta specs）
   - `design.md`（如果存在）
   - `tasks.md`（如果存在）

3. **先运行 validate（快速信号）**

   运行：
   - `llman sdd-legacy validate <id> --strict --no-interactive`

   若校验失败，将其记录为 **CRITICAL**（附原始错误/输出）。

4. **检查 Completeness（完整性）**

   - 若存在 `tasks.md`，把所有未勾选任务（`- [ ]`）列为 **CRITICAL**。
   - 若不存在任何 delta specs，把它列为 **CRITICAL**（无法验证需求覆盖）。

5. **检查 Correctness（正确性）**

   对每条 requirement 与 scenario：
   - 在代码中寻找实现证据（文件/符号），并记录
   - 对可能不一致之处给出 **WARNING**（附具体修复建议）
   - 若场景缺少测试，建议补测试（或说明为何不做测试）

6. **检查 Coherence（一致性）**

   - 若存在 `design.md`，验证实现是否遵循关键决策；否则建议更新代码或更新 `design.md` 使其反映真实实现。
   - 检查新增代码是否符合仓库约定（结构、命名、错误处理等）。

7. **输出简短验证报告**

   输出：
   - **CRITICAL**（归档前必须修复）
   - **WARNING**（建议修复）
   - **SUGGESTION**（可选优化）

   最后给出下一步：
   - 若存在 CRITICAL：建议用 `/llman-sdd:apply <id>` 修复
   - 若通过：建议用 `/llman-sdd:archive <id>` 归档

**护栏**
- 不要编造证据：引用具体文件路径与观察结果
- 建议保持小而可执行
- verify 模式优先输出报告与下一步，而不是直接实施修复

{{ unit("skills/structured-protocol") }}
