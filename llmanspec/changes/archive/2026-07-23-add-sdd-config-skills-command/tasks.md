# Tasks: Add `llman sdd config` interactive command

> 按依赖顺序，每项垂直切片。

## Seams under test

- [x] seam: CLI 子进程 `llman sdd config` / `config skills`（confirmed）
- [x] seam: `OPTIONAL_SKILL_NAMES` ↔ `OPTIONAL_SKILL_FILES` ↔ 模板 三者同步（confirmed）

## 阶段 1：纳入 3 个新 skill（解锁候选集）

- [x] task-1: config.rs `OPTIONAL_SKILL_NAMES` + templates.rs `OPTIONAL_SKILL_FILES` 各加 3 项 + DEFAULT_CONFIG_* 注释示例 [blocked-by: none]
- [x] task-2: 新增 6 个模板（en+zh-Hans 的 arch-review/wayfinder/research），从 .agents/skills/ 实例回填为 MiniJinja 模板 [blocked-by: task-1]
- [x] task-3: 重生成 schema + `just check-sdd-templates` 通过 [blocked-by: task-2]

## 阶段 2-3：config skills 命令

- [x] task-4: command.rs 加 Config(SddConfigArgs) 变体 + SddConfigCommands::Skills + dispatch_config（仿 Index 模式） [blocked-by: task-3]
- [x] task-5: 新增 config_skills.rs：MultiSelect 交互 + 写回 + --no-interactive 降级 + --json [blocked-by: task-4]
- [x] task-6: mod.rs 声明 config_skills 模块 + `cargo build` 通过 [blocked-by: task-5]

## 阶段 4：config 总览

- [x] task-7: dispatch_config 中 command=None 时打印 config 总览 [blocked-by: task-6]

## 阶段 5：测试 + 收尾

- [x] task-8: 集成测试 test_sdd_config_skills_non_interactive + bdd_compat smoke 列表加 config [blocked-by: task-7]
- [x] task-9: `just check` + `just check-sdd-templates` + `llman sdd validate --all` 全绿 [blocked-by: task-8]
- [x] task-10: `llman sdd change finalize add-sdd-config-skills-command` [blocked-by: task-9]
