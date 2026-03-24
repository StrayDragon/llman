use crate::config::{CLAUDE_CODE_APP, CODEX_APP, CURSOR_APP, Config};
use crate::fs_utils::atomic_write_with_mode;
use crate::path_utils::validate_path_segment;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::PathBuf;

fn validate_app(app: &str) -> Result<()> {
    match app {
        CURSOR_APP | CODEX_APP | CLAUDE_CODE_APP => Ok(()),
        _ => Err(anyhow!(t!("errors.invalid_app", app = app))),
    }
}

pub fn list_templates(config: &Config, app: &str) -> Result<Vec<String>> {
    validate_app(app)?;
    config.list_rules(app)
}

pub fn read_template(config: &Config, app: &str, name: &str) -> Result<String> {
    validate_app(app)?;
    let name = validate_path_segment(name, "template name")
        .map_err(|e| anyhow!("invalid template name: {e}"))?;
    let template_path = config.rule_file_path(app, &name);
    if !template_path.exists() {
        return Err(anyhow!(t!("errors.rule_not_found", name = name)));
    }
    fs::read_to_string(&template_path).map_err(|e| {
        anyhow!(t!(
            "errors.file_read_failed",
            path = template_path.display(),
            error = e
        ))
    })
}

pub fn upsert_template(config: &Config, app: &str, name: &str, content: &str) -> Result<PathBuf> {
    validate_app(app)?;
    let name = validate_path_segment(name, "template name")
        .map_err(|e| anyhow!("invalid template name: {e}"))?;

    config.ensure_app_dir(app)?;
    let template_path = config.rule_file_path(app, &name);
    atomic_write_with_mode(&template_path, content.as_bytes(), None)?;
    Ok(template_path)
}

pub fn remove_template(config: &Config, app: &str, name: &str) -> Result<()> {
    validate_app(app)?;
    let name = validate_path_segment(name, "template name")
        .map_err(|e| anyhow!("invalid template name: {e}"))?;
    let template_path = config.rule_file_path(app, &name);

    if !template_path.exists() {
        return Err(anyhow!(t!("errors.rule_not_found", name = name)));
    }

    fs::remove_file(&template_path)?;
    Ok(())
}

pub fn build_llman_prompts_body(
    config: &Config,
    app: &str,
    templates: &[String],
) -> Result<String> {
    validate_app(app)?;
    if templates.is_empty() {
        return Ok(String::new());
    }

    let mut parts = Vec::new();
    for name in templates {
        let name = validate_path_segment(name, "template name")?;
        let content = read_template(config, app, &name)?;
        parts.push(format!(
            "## llman prompts: {name}\n\n{}",
            content.trim_end()
        ));
    }
    Ok(parts.join("\n\n"))
}
