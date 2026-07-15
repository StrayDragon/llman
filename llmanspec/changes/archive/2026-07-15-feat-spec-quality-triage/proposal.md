# Proposal: Spec Quality & Agent Triage — 健康检测与分流规则

## Why

两个问题：

### 问题 A：Spec 健康退化
没有系统机制发现「坏口味」spec：
- **僵尸 req**：代码已不存在，spec 还写着 MUST
- **迷雾 spec**：purpose 写「提升体验」但没有可验证的 MUST/SHALL
- **范围膨胀**：valid_scope 覆盖整个仓库但只测了一个函数
- **重复规范**：两个 spec 描述同一行为合约
- **过时**：6 个月未更新，可能已不反映现实

agent 无法知道一个 spec 是否可信，被迫全信或全不信。

### 问题 B：Agent 无分流能力
当前所有 skill 引导 agent 走「完整 SDD 流程」——即使 typo fix 也要 proposal → delta → apply → archive。

缺少一个**变更规模判断**（triage）步骤，导致大量 token 浪费在流程脚手架而非实际改动上。

## What Changes

### 1. Spec Health Detection

集成到 `llman sdd validate`（或新增 `--health` 标志），检测以下坏口味：

| 检测 | 方法 | 严重度 |
|------|------|--------|
| **僵尸 req** | requirement 的 statement 提及的行为，在代码中搜索不到对应实现（通过 grep 关键词 / 调用栈扫描） | critical |
| **迷雾 req** | requirement 缺少 scenario 覆盖，或 scenario 数量 < 阈值（建议 ≥ 1） | warn |
| **范围膨胀** | valid_scope 覆盖目录但 spec 只有 1 个 req，或与同级 spec 严重重叠 | warn |
| **过时** | staleness 信号 + `git log --since` 检查最后修改时间 > 6 个月 | info |
| **重复** | 与其他 spec 的 `valid_scope` 和 `purpose` 关键词语义重叠 | info |
| **无效** | 无法解析为合法 TOON/ISON 结构 | error |

输出示例：
```
$ llman sdd validate --health

errors-exit         2 reqs  ✅ fresh    — OK
cli-experience     10 reqs  ⚠️ stale   最后一次修改: 2025-12 (7 months ago)
tool-clean-comments 4 reqs  ⚠️ zombie  r2 引用的函数已不存在
sdd-workflow       46 reqs  ❌ bloated 建议压缩（> 30 reqs 且场景覆盖率仅 30%）
```

这些健康信号也暴露到 `list --specs --json` 的 `health` 字段中，agent 一次读取即可知道哪些 spec 可信。

### 2. Agent Triage 规则（Prompt 层面）

在所有入口 skill（onboard / explore / propose / apply）中加入**变更规模判断**步骤：

```
## 变更规模判断（Triage）

在走完整 SDD 流程或直接修改前，先判断变更的性质：

### 行为合约变更（必须走完整流程）
变更改变了某个 spec 的 MUST/SHALL 定义的**外部可观测行为**。
→ 必须完整 SDD 流程（proposal + delta spec + tasks + archive）
例：修改退出码、删除命令、改变输出格式

### 实现变更（可走快速路径）
变更不改变外部行为，只改变内部实现。
→ 直接改代码，无需 change 目录
例：重构、修 typo、提取函数、加测试

### 治理/工具变更（轻量路径）
变更修改 CI/工具配置。
→ 仅创建 proposal.md，无需 delta spec
例：更新 clippy 规则、改 justfile

### 元规范变更（必须走完整流程）
变更修改 SDD 规范/模板/流程本身。
→ 必须走完整 SDD 流程（自举）
例：改 AGENTS.md 模板、改 skill 内容
```

### 3. Quick Path Skill 模板

新增 `llman-sdd-quick` skill（或融入 `explore`），供 agent 走快速路径时使用：

```
## LLMAN SDD Quick Path

对于不涉及行为合约变更的小改动使用此路径。

### 步骤
1. 确认变更属于「实现变更」或「spec 维护」
2. 直接修改代码
3. 直接修改主 spec（`llmanspec/specs/<cap>/spec.toon`）
4. 运行 `llman sdd validate --specs` 确保 spec 仍合法
5. git commit（message 写明 why）
6. 无需 change 目录，无需 archive
```

### 4. 直接改 spec 的质量保证

对于快速路径中直接修改主 spec 的场景，需要保证：

- **改前改后都通过 `validate`**（前置 + 后置校验）
- **scope 不变**：快速路径不允许修改 valid_scope（scope 变更 = 结构性变更）
- **diff 可追溯**：commit message 必须包含 spec 变更原因
- **health 反弹**：修改后 spec 的 stale 标记会刷新到 fresh

## Capabilities

- `sdd-workflow`: triage 规则与 quick path 加入 workflow
- `cli`: `validate --health` 坏口味检测
- `prompts-management`: skill 模板更新（onboard / explore / propose / apply + 新增 quick）
- `sdd-structured-skill-prompts`: structured protocol 增加 triage + context-first 约束

### Skill 模板调整细案（5 个文件的具体改动）

#### 改动 1: `llman-sdd-onboard.md`

```diff
## 步骤
1. 阅读 `llmanspec/config.yaml` 了解项目上下文、约定与规则。
-2. 查看当前的变更与 specs。
+2. 使用 `llman sdd context --task "<任务描述>" --paths "<路径>"` 获取相关 specs。
+3. 如果返回 `quality: "unavailable"`，按以下分流处理：
+   ├─ 任务不强制语义检索 → 启动 `llman sdd index rebuild --async` 后台重建
+   │    ├─ 用 `llman sdd list --specs --json` 继续工作
+   │    └─ context 就绪后自动生效，无需手动检查
+   └─ 任务必须语义检索 → 启动前台重建，记录 PID
+        ├─ `llman sdd index rebuild`（等待完成）
+        └─ 或 `llman sdd index rebuild --async` + 记录 PID
+            后用 `llman sdd index rebuild --check` 轮询
+4. 根据 context 的 `direct`/`related` 分类，只读 target spec 全文。
+3. 根据 context 的 `direct`/`related` 分类，只读 target spec 全文。
-3. 按照 提案 -> 实施 -> 归档 的流程推进。
+4. 判断变更规模（见 triage 规则），决定走完整 SDD 流程或快速路径。
-4. 使用 `llman sdd graph` 可视化变更依赖关系（depends_on/blocks）。
+5. 按照 提案 -> 实施 -> 归档 的流程（完整路径）或直接修改（快速路径）推进。
+6. 使用 `llman sdd graph` 可视化变更依赖关系（depends_on/blocks）。
```

#### 改动 2: `llman-sdd-explore.md`

```diff
## 建议动作
-1. 澄清目标与约束（问 1–3 个问题）。
-2. 先看上下文：`llman sdd list --json`
-3. 如果某个 change id 相关，阅读 `llmanspec/changes/<id>/` 下的 artifacts。
-4. 探索 2–3 个选项与权衡。
+1. 使用 `llman sdd context --task "<任务>" --paths "<文件>"` 快速定位相关 specs。
+2. 阅读 context 的 `direct` 列出的 spec 全文（这些是必须理解的合约）。
+3. 如果某个 change id 相关，阅读 `llmanspec/changes/<id>/` 下的 artifacts。
+4. 探索 2–3 个选项与权衡。
+5. 判断变更规模（triage），确定是否需要走完整 SDD 流程。

## 退出探索模式
 当用户准备开始实现时，建议：
- `llman-sdd-propose`（提出提案并生成工件）
- `llman-sdd-new-change`（创建 change）
- `llman-sdd-ff`（一次性创建所有 artifacts）
- `llman-sdd-apply`（按 tasks 实施）
+- `llman-sdd-quick`（快速路径：小改动直接改）
 若用户在探索模式中要求你开始实现，STOP 并提醒其先退出探索模式。
```

#### 改动 3: `llman-sdd-propose.md`（新增 triage + async 步骤）

```diff
 ## 步骤
-1. 收集输入：
+1. 判断变更规模（triage）：
+   - 行为合约变更（改 MUST/SHALL、改外部行为）→ 走完整 SDD 流程
+   - 实现变更（重构、typo、性能）→ 建议走快速路径，用 `llman-sdd-quick`
+   - 元规范变更（改 SDD 模板/流程）→ 必须走完整 SDD 流程
+2. 使用 `llman sdd context --task "<目标>" --paths "<范围>"` 获取相关 specs。
+   如果 context 不可用，参考 onboard 的异步重建分流处理。
+3. 收集输入：
```

#### 改动 4: `llman-sdd-apply.md`（新增 context-first 步骤）

```diff
 ## 步骤
-1. 选择变更 id：
+1. 使用 `llman sdd context --task "<task from proposal>" --paths "<scope from spec>"` 确认相关 specs。
+   如果 context 不可用，启动后台重建继续工作。
+2. 选择变更 id：
```

#### 改动 5: 新增 `llman-sdd-quick.md`

```markdown
---
name: "llman-sdd-quick"
description: "小变更快速路径：不涉及行为合约时直接修改代码。"
---

# LLMAN SDD Quick Path

对于不涉及行为合约变更的小改动使用此路径。

## 使用条件（所有条件必须满足）
- 不改变任何 spec 中 MUST/SHALL 定义的外部可观测行为
- 不涉及跨 capability 的修改
- 不涉及迁移/兼容性
- 不是 SDD 元规范变更

## 步骤
1. 用 `llman sdd context --task "..." --paths "..."` 确认无相关 spec 变更需要
   - 如果 context 返回 quality: unavailable → 启动 `llman sdd index rebuild --async` 后台重建
   - 同时用 `llman sdd list --specs --json` 或直接探索代码继续工作
   - context 就绪后自动生效，无需主动检查
   - 如果必须等待语义结果 → `llman sdd index rebuild`（前台等待 ~30s）
2. 直接修改代码
3. 如果涉及 spec 的维护性调整（修错字、收紧 scope），直接编辑 spec 文件并用 `llman sdd validate --specs` 校验
4. git commit（message 写明 why）
5. 无需 change 目录，无需 archive

## 边界处理
- 如果在修改中发现需要改变行为合约 → STOP，改走 `llman-sdd-propose`
- 如果涉及到多个文件且不确定 scope → 先用 `llman sdd context` 确认

{{ unit("skills/sdd-commands") }}

{{ unit("skills/structured-protocol") }}
```

#### 改动 6: `sdd-commands.md`（命令单元模板扩展）

```diff
 常用命令：
+- `llman sdd context --task "<description>" --paths "<files>"`（获取相关 specs，统一入口）
+- `llman sdd index rebuild`（重建 embedding 索引）
+- `llman sdd index rebuild --check`（检查索引新鲜度）
```

#### 改动 7: `structured-protocol.md`（结构化协议）

```diff
 ## Constraints
 - 变更保持最小化且范围明确。
 - 标识符或意图不明确时禁止猜测。
+- 在读取 spec 全文前，先使用 `llman sdd context --task --paths` 获取相关 specs。
+- 判断变更规模后选择路径：行为合约变更走完整 SDD，实现变更走快速路径。

 ## Workflow
 - 以 `llman sdd` 命令结果为事实来源。
 - 涉及文件/规范变更时执行校验。
+- 首选 `llman sdd context` 获取相关 specs，而非全量读取或猜测。
```

### 调整后 agent 的工作流对比

```
调整前:                             调整后:

AGENT 启动                      AGENT 启动
  ├─ 读 AGENTS.md                  ├─ 读 AGENTS.md
  ├─ 读 config.yaml                ├─ 读 config.yaml
  ├─ llman sdd list                ├─ llman sdd context --task "..." --paths "..."
  ├─ 猜哪些 spec 相关              │   ← 一次调用就知道 direct/related/unrelated
  ├─ 读 3-5 个 spec 全文           ├─ 只读 direct spec 全文（通常 1-2 个）
  ├─ 决定：走完整 SDD              ├─ triage：行为合约？→ 完整 SDD
  ├─ 创建 proposal/specs/tasks     │         实现变更？→ quick path
  ├─ 实现                         ├─ 直接改代码 / 创建 change
  └─ archive                       └─ git commit / archive
```

### 模板调整的风险与回退

- **新增字段风险**：在 `structured-protocol.md` 加约束后，所有 skill 模板都会引入新的 triage 要求。如果 agent 行为异常，可回滚到旧版约束。
- **Quick Path 滥用风险**：agent 可能用 quick path 跳过本应走完整流程的变更。缓解方式：quick path 的 use condition 写得足够严格（不改 MUST/SHALL），且 agent 遇到边界模糊时必须升级。
- **context 命令不可用风险**：如果 context 返回 `quality: unavailable`，agent 回退到 `list --specs --json` 的传统方式（有 purpose/scope 字段，比之前好）。

## Async Rebuild 在 skill 中的完整决策树

以下是 agent 在各个 skill 中遇到 `context` 不可用时的统一决策指引：

```
llman sdd context --task "..." --paths "..."
    │
    ├─ quality: semantic → 正常使用 direct/related
    │
    ├─ quality: keyword → index 过期，结果有限但可用
    │    └─ 顺便启动 `llman sdd index rebuild --async`
    │
    └─ quality: unavailable → index 不存在
         │
         ├─ 任务不强制需要语义检索（大多数场景）
         │    ├─ 启动 `llman sdd index rebuild --async`
         │    │   输出 PID，后台执行
         │    ├─ 同时用 `llman sdd list --specs --json` 继续工作
         │    │   虽然只有 keyword 级别的 metadata，但足够
         │    ├─ 完成后 context 会自动检测到 index，无需手动操作
         │    └─ 如果重建超时或失败（`--check` 显示 stale）：
         │         └─ 降级为只读 spec 文件：`cat llmanspec/specs/*/spec.toon`
         │
         └─ 任务必须语义检索
              ├─ `llman sdd index rebuild`（前台等待）
              │   或 `llman sdd index rebuild --async`（后台 + 记录 PID）
              ├─ 用 `llman sdd index rebuild --check` 轮询进度
              ├─ 约 30 秒后重试 `llman sdd context --task "..." --paths "..."`
              └─ 如果重建失败：
                   └─ 报告失败原因，建议检查 embedding API 配置
```

### Skill 模板中的具体写法示例

在 `onboard.md` / `explore.md` / `propose.md` / `apply.md` 中统一使用以下注记块：

```markdown
> **当 context 不可用时**：
> 1. 启动后台重建：`llman sdd index rebuild --async`（~30s，PID 显示在输出中）
> 2. 用 `llman sdd list --specs --json` 继续工作（keyword 级别，够用）
> 3. 如果必须语义匹配（跨 capability 融合）：前台重建或用 `--check` 轮询
> 4. 索引就绪后 context 自动生效
```

## 待定问题

1. **僵尸 req 检测的可靠性**：grep 关键词只能给出「疑似」信号，如何避免 false positive？
   - 建议：检测结果标记为 `suspected-zombie` 而非 `confirmed-zombie`，列出匹配证据供人类判断
2. **Quick Path 与完整流程的边界模糊**：如果实现变更中发现需要改合约怎么办？
   - 建议：agent 遇到边界模糊时，升级为完整 SDD 流程（等同于 prompt 中「高影响歧义必须先澄清」）
3. **是否需要一个 `llman sdd analyse` 命令**来专门做健康分析？
   - 建议：v1 集成到 `validate --health`，v2 如果坏口味检测逻辑复杂再拆独立命令
