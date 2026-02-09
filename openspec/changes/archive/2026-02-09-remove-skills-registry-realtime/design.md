## Context

当前 `llman skills` 在实现上存在两套状态来源：
1) 目标目录中的真实 symlink（运行时状态）；
2) `<skills_root>/registry.json` 中的 `skills.targets` 与 `presets`（持久化状态）。

这会带来行为理解困难与维护成本：交互默认值依赖文件系统，非交互同步依赖 registry，预设来源优先 registry 再回退目录推断。对于“以当前文件系统为准”的用户心智，这种混合模型复杂且易产生漂移。

该变更将状态模型收敛为“实时计算优先”，并显式下线 registry 依赖。

## Goals / Non-Goals

**Goals:**
- 移除 `llman skills` 对 registry 文件的读取、写入与原子写保障分支。
- 将技能启用判断统一为运行时计算：目标目录链接状态 + `config.toml` 默认值。
- 将 presets 统一为目录分组推断（`<group>.<name>`），不再支持 `registry.presets` 配置化预设。
- 保证交互取消/冲突取消仍是安全 no-op，不产生半完成变更。

**Non-Goals:**
- 不引入新的 presets 持久化位置（如 `config.toml` 新字段）作为本次范围。
- 不新增 presets 命令行参数。
- 不扩展 Windows 行为保证范围（仍保持当前“非目标平台”定位）。

## Decisions

### 1) 状态源收敛为文件系统实时状态

`is_skill_linked(skill, target)` 成为跨交互/非交互的统一真实状态来源。

- 交互模式：默认勾选继续从真实链接状态计算（保持既有体验）。
- 非交互模式：不再读取 registry 期望状态，改为按以下实时规则计算每个 `<skill, target>` 的 desired：
  - 若当前已链接：`desired = true`（保留当前显式状态）；
  - 若当前未链接：`desired = target.enabled`（仅在未链接时应用配置默认值）。

这样可保持“文件系统是事实来源”，并避免每次运行覆盖用户已存在的显式链接选择。

**Alternatives considered**
- A. 每次非交互都强制 `desired = target.enabled`：简单但会覆盖用户当前链接状态，侵入性高。
- B. 保留 registry 但只存 `skills.targets`：仍保留双状态源，未解决根因。

### 2) 预设来源仅保留目录推断

移除 `registry.presets -> runtime_presets` 路径，保留按目录名自动推断分组：
- `<group>.<name>` 归入 `<group>`；
- 无 `.` 归入 `ungrouped`。

**Alternatives considered**
- A. 将 `registry.presets` 迁入 `config.toml`：可行，但引入 schema/迁移与配置设计复杂度，不适合本次“快速去状态化”目标。

### 3) 兼容策略：软忽略旧 registry，不做自动迁移

本次不做旧 `registry.json` 到新模型的数据迁移。程序将不再读取或写入该文件；若文件存在则视为历史残留。

配套更新文档与规范，明确：
- `registry.presets` 失效；
- 需要通过目录命名实现分组。

### 4) 模块收敛与类型瘦身

`SkillsPaths` 移除 `registry_path` 字段；`skills::catalog::registry` 模块从主流程解绑（可删除或保留过渡占位，取决于实现阶段）。

## Risks / Trade-offs

- [用户依赖 registry.presets] → 分组会退化为目录推断；通过文档与 release note 明确 BREAKING，并给目录命名迁移示例。
- [非交互语义变化] → 通过规范明确新规则（已链接优先，未链接回退 `target.enabled`），并补充集成测试覆盖。
- [历史脚本依赖 registry.json 存在] → 提供“文件不再维护”的说明，避免脚本误判为 bug。

## Migration Plan

1. 更新 `skills-management` 规范，删除/替换所有 registry 依赖要求。
2. 代码实现分两步：
   - 第一步：移除读写路径，保留现有交互流程与冲突处理。
   - 第二步：清理 registry 模块与测试、文档残留。
3. 更新文档：
   - `README.md` 与 `docs/skills-registry-presets.md` 改为“实时分组推断说明”。
4. 回滚策略：
   - 若发现不可接受回归，可回滚到上一版本恢复 registry 行为；本次不引入不可逆数据迁移。

## Open Questions

- 是否在后续变更中为“可配置 presets”引入新承载（例如 `config.toml`）？本次建议不做，先完成状态源收敛。
