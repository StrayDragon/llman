# Design: add-skills-presets

## Goals

- 仅通过 `llman skills` 的交互流程提供预设能力
- 不引入新的 presets CLI 参数
- 支持两类预设来源：
  - 用户在 `registry.json` 手工维护的 `presets`
  - 未配置 `presets` 时，从技能目录名自动推断的默认预设
- 在进入交互前完成预设校验并 fail-fast

## Non-Goals

- 不提供 `--preset` / `--save-preset` / `--list-presets` / `--delete-preset`
- 不提供预设 CRUD API
- 不自动写回推断得到的默认预设到 `registry.json`

## Architecture Overview

```
llman skills
   │
   ├─ 1) load config/registry + discover skills
   │
   ├─ 2) build preset catalog (runtime)
   │      a) if registry.presets is non-empty: use registry presets
   │      b) else infer defaults from <preset>.<skill> directory naming
   │
   ├─ 3) validate presets (fail-fast)
   │      - parent exists
   │      - no circular extends
   │      - referenced skill_dirs exist
   │
   └─ 4) interactive flow
          - Select mode: Apply preset / Select individually / Exit
          - Apply preset: preset -> agent -> scope -> confirm
          - Select individually: keep existing behavior
```

## Data Structures

### Registry extension

```rust
// src/skills/catalog/registry.rs

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Registry {
    pub skills: HashMap<String, SkillEntry>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub presets: HashMap<String, PresetEntry>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PresetEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,
    /// skill directory names, not skill_id
    pub skill_dirs: Vec<String>,
}
```

### Runtime preset model

```rust
pub struct RuntimePreset {
    pub name: String,
    pub description: Option<String>,
    pub extends: Option<String>,
    pub skill_dirs: Vec<String>,
    pub source: PresetSource,
}

pub enum PresetSource {
    Registry,
    Inferred,
}
```

## Preset source rules

1. 当 `registry.presets` 非空时，使用其作为唯一预设来源。  
2. 当 `registry.presets` 为空时，扫描技能目录名：
   - 目录名匹配 `<preset>.<skill>` 时，归入 `<preset>` 预设
   - `skill_dirs` 保存完整目录名（如 `superpowers.brainstorming`）
3. 自动推断仅存在于运行时，不落盘。

## Validation rules (before any prompt)

在命令进入交互菜单前，必须一次性完成以下校验：

1. `extends` 指向的父预设必须存在  
2. 继承图不得有环  
3. 每个 `skill_dirs` 必须能映射到已发现的技能目录名  
4. 预设解析后的技能集合不能为空

若任一校验失败：
- 命令立即返回错误
- 不进入任何交互 prompt
- 错误消息包含预设名和失败原因

## Key algorithms

### Build runtime presets

```rust
fn build_runtime_presets(
    registry: &Registry,
    skills: &[SkillCandidate],
) -> HashMap<String, RuntimePreset> {
    if !registry.presets.is_empty() {
        return registry_presets_to_runtime(&registry.presets);
    }
    infer_presets_from_skill_dirs(skills)
}
```

### Infer defaults from directory names

```rust
fn infer_presets_from_skill_dirs(skills: &[SkillCandidate]) -> HashMap<String, RuntimePreset> {
    let mut out = HashMap::new();
    for skill in skills {
        let Some(dir) = skill.skill_dir.file_name().and_then(|n| n.to_str()) else { continue };
        let Some((preset, _tail)) = dir.split_once('.') else { continue };
        let entry = out.entry(preset.to_string()).or_insert_with(|| RuntimePreset {
            name: preset.to_string(),
            description: None,
            extends: None,
            skill_dirs: vec![],
            source: PresetSource::Inferred,
        });
        if !entry.skill_dirs.iter().any(|it| it == dir) {
            entry.skill_dirs.push(dir.to_string());
        }
    }
    out
}
```

### Resolve preset (with extends)

```rust
fn resolve_preset(
    presets: &HashMap<String, RuntimePreset>,
    name: &str,
    visiting: &mut Vec<String>,
) -> Result<Vec<String>> {
    if visiting.iter().any(|n| n == name) {
        return Err(anyhow!("Circular preset dependency detected: {}", visiting.join(" -> ")));
    }
    let preset = presets
        .get(name)
        .ok_or_else(|| anyhow!("Preset '{}' not found", name))?;

    visiting.push(name.to_string());
    let mut out = Vec::<String>::new();

    if let Some(parent) = &preset.extends {
        out.extend(resolve_preset(presets, parent, visiting)?);
    }

    for dir in &preset.skill_dirs {
        if !out.iter().any(|it| it == dir) {
            out.push(dir.clone());
        }
    }

    visiting.pop();
    Ok(out)
}
```

## Interactive flow

### Mode selection

```
? Select mode:
> Apply preset
  Select individually
  Exit
```

- 当运行时预设列表为空时，不展示 `Apply preset`。

### Apply preset path

```
Apply preset
  -> Select preset
  -> Select agent
  -> Select scope
  -> Confirm and apply
```

- 默认勾选来自“已解析预设中的 skill_id 集合”。
- 同步仍沿用现有 target diff 机制。

### Select individually path

- 保持既有交互能力。
- 列表展示按目录名前缀分组聚合（`group.name` 归入 `group`，无前缀归入 `ungrouped`）。

## Error handling

| 场景 | 错误消息（示例） |
|---|---|
| extends 父预设不存在 | `Preset 'full-stack' extends missing preset 'daily'.` |
| 继承循环 | `Circular preset dependency detected: A -> B -> A.` |
| 预设引用不存在技能目录 | `Preset 'daily' references unknown skill dir 'x.y'.` |
| 预设为空 | `Preset 'daily' resolves to an empty skill set.` |

## Compatibility

- `registry.presets` 使用 `#[serde(default)]`，旧文件兼容
- 无 `presets` 时不会写出空字段
- 推断得到的默认预设为运行时数据，不修改 `registry.json`
