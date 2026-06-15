# 设计文档：自动清理 extra_skills 技能文件

## 概述

优化 `llman sdd init --update` 命令，使其在更新技能文件时自动清理不再需要的可选技能目录。

## 当前行为

```rust
fn write_tool_skills(base: &Path, templates: &[super::templates::SkillTemplate]) -> Result<()> {
    fs::create_dir_all(base)?;
    for template in templates {
        let dir_name = template.name.trim_end_matches(".md");
        let skill_dir = base.join(dir_name);
        fs::create_dir_all(&skill_dir)?;
        let skill_path = skill_dir.join("SKILL.md");
        atomic_write_with_mode(&skill_path, template.content.as_bytes(), None)?;
    }
    Ok(())
}
```

当前实现只写入技能文件，不删除已存在的技能目录。

## 设计方案

### 1. 新增清理函数

```rust
fn cleanup_stale_skills(base: &Path, templates: &[super::templates::SkillTemplate]) -> Result<()> {
    // 获取期望的技能目录名列表
    let expected_skills: HashSet<String> = templates
        .iter()
        .map(|t| t.name.trim_end_matches(".md").to_string())
        .collect();

    // 获取可选技能列表（用于安全过滤）
    let optional_skills: HashSet<&str> = OPTIONAL_SKILL_FILES
        .iter()
        .map(|name| name.trim_end_matches(".md"))
        .collect();

    // 扫描已存在的技能目录
    if !base.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(base)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();

        // 只清理可选技能，不清理核心技能或用户自定义技能
        if !optional_skills.contains(dir_name.as_str()) {
            continue;
        }

        // 如果技能不在期望列表中，删除它
        if !expected_skills.contains(&dir_name) {
            let skill_path = entry.path();
            fs::remove_dir_all(&skill_path)?;
            // 输出日志提示
            eprintln!("Cleaned up stale skill: {}", dir_name);
        }
    }

    Ok(())
}
```

### 2. 修改 write_tool_skills 函数

在写入技能文件前，先调用清理函数：

```rust
pub(crate) fn run_with_root(root: &Path) -> Result<()> {
    let llmanspec_path = root.join(LLMANSPEC_DIR_NAME);
    if !llmanspec_path.exists() {
        let cmd = "llman sdd init";
        return Err(anyhow!(t!("sdd.update_skills.no_llmanspec", cmd = cmd)));
    }

    let config = load_or_create_config(&llmanspec_path)?;

    let templates = skill_templates(&config, root)?;
    enforce_ethics_governance(&templates)?;
    let skills_base = root.join(".agents").join("skills");

    // 新增：清理不再需要的技能
    cleanup_stale_skills(&skills_base, &templates)?;

    // 写入技能文件
    write_tool_skills(&skills_base, &templates)?;

    Ok(())
}
```

### 3. 安全保护机制

1. **只清理可选技能**：通过 `OPTIONAL_SKILL_FILES` 列表过滤，确保不会误删核心技能或用户自定义技能。
2. **目录存在性检查**：在清理前检查目录是否存在。
3. **错误处理**：清理失败时返回错误，但不影响后续的技能写入。

### 4. 日志输出

清理时输出 INFO 级别日志，告知用户哪些技能被清理。可以使用 `eprintln!` 或者项目中的日志机制。

## 测试策略

1. **单元测试**：测试清理函数在各种场景下的行为。
2. **集成测试**：测试完整的更新流程，包括清理和写入。

## 向后兼容性

- 不改变现有 API 接口
- 不影响核心技能的加载
- 只是增加了自动清理能力
