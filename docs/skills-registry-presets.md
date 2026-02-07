# Skills `registry.json` 分组配置说明

本文档说明如何在 `<skills_root>/registry.json` 中配置 skills 分组（presets）。

## 文件位置

- 默认路径：`~/.config/llman/skills/registry.json`
- 若设置了 `LLMAN_SKILLS_DIR` 或 `--skills-dir`，则使用对应目录下的 `registry.json`

## 顶层结构

`registry.json` 目前包含两个主要字段：

- `skills`：按 `skill_id` 记录各 target 的启用状态
- `presets`：分组配置（用于 `llman skills` 交互树中的分组父节点）

示例：

```json
{
  "skills": {
    "brainstorming": {
      "targets": {
        "claude_user": true,
        "codex_repo": false
      },
      "updated_at": "2026-02-07T12:34:56Z"
    }
  },
  "presets": {
    "daily": {
      "description": "常用基础技能",
      "skill_dirs": [
        "superpowers.brainstorming",
        "mermaid-expert"
      ]
    },
    "full-stack": {
      "extends": "daily",
      "skill_dirs": [
        "astral-sh.ruff",
        "astral-sh.uv"
      ]
    }
  }
}
```

## `presets` 字段说明

每个分组对象支持以下字段：

- `description`（可选）：显示在分组名称后
- `extends`（可选）：继承另一个分组
- `skill_dirs`（必填，允许为空数组但会被运行前校验拒绝）：技能目录名列表

注意：`skill_dirs` 写的是“技能目录名”，不是 frontmatter 的 `name`，也不是 `skill_id`。

例如目录是：

- `superpowers.brainstorming`
- `op7418.Humanizer-zh`

就应写：

```json
"skill_dirs": ["superpowers.brainstorming", "op7418.Humanizer-zh"]
```

## 分组校验规则（执行 `llman skills` 时）

命令在进入交互前会校验：

- `extends` 指向的父分组必须存在
- 继承链不能有环
- `skill_dirs` 必须是当前 `<skills_root>` 下可识别的目录名
- 分组解析后的技能集合不能为空

任一失败都会直接报错并退出，不会进入交互页面。

## 未配置 `presets` 时的行为

当 `registry.json` 没有 `presets` 或其为空时，程序会自动按目录名推断分组：

- `<group>.<name>` 会归入 `<group>`
- 无 `.` 的目录会归入 `ungrouped`

该自动推断仅用于运行时展示，不会回写到 `registry.json`。

## 建议维护方式

- 推荐用你熟悉的 JSON 编辑器手动维护 `registry.json`
- 修改后先执行一次 `llman skills`，快速确认分组是否通过校验
- 如果只是想用目录自动分组，可以不写 `presets`

