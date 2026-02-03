use super::config::{SddConfig, locale_fallbacks};
use super::regions::expand_regions;
use crate::sdd::shared::constants::LLMANSPEC_MARKERS;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

pub struct TemplateFile {
    pub name: &'static str,
    pub content: String,
}

pub struct SkillTemplate {
    pub name: &'static str,
    pub content: String,
}

const SPEC_DRIVEN_FILES: &[&str] = &["proposal.md", "spec.md", "design.md", "tasks.md"];

const SKILL_FILES: &[&str] = &[
    "llman-sdd-onboard.md",
    "llman-sdd-new-change.md",
    "llman-sdd-show.md",
    "llman-sdd-validate.md",
    "llman-sdd-archive.md",
];

pub fn spec_driven_templates(config: &SddConfig, root: &Path) -> Result<Vec<TemplateFile>> {
    let mut files = Vec::new();
    for name in SPEC_DRIVEN_FILES {
        let content = load_template(config, root, &format!("spec-driven/{}", name))?;
        files.push(TemplateFile { name, content });
    }
    files.sort_by_key(|f| f.name);
    Ok(files)
}

pub fn skill_templates(config: &SddConfig, root: &Path) -> Result<Vec<SkillTemplate>> {
    let mut files = Vec::new();
    for name in SKILL_FILES {
        let content = load_template(config, root, &format!("skills/{}", name))?;
        files.push(SkillTemplate { name, content });
    }
    Ok(files)
}

pub fn render_project_template(
    project_name: &str,
    config: &SddConfig,
    root: &Path,
) -> Result<String> {
    let base = load_template(config, root, "project.md")?;
    Ok(base
        .replace("{{projectName}}", project_name)
        .replace("{{description}}", "TODO: Describe project purpose")
        .replace("{{techStack}}", "TODO: List key technologies"))
}

pub fn managed_block_content(config: &SddConfig, root: &Path) -> Result<String> {
    load_template(config, root, "agents.md")
}

pub fn root_stub_content(config: &SddConfig, root: &Path) -> Result<String> {
    load_template(config, root, "agents-root-stub.md")
}

pub fn default_agents_file(config: &SddConfig, root: &Path) -> Result<String> {
    let block = managed_block_content(config, root)?;
    Ok(format!(
        "{}\n{}\n{}\n\n## Project Notes\n\n- Add project-specific guidance here.\n",
        LLMANSPEC_MARKERS.start, block, LLMANSPEC_MARKERS.end
    ))
}

fn load_template(config: &SddConfig, root: &Path, relative_path: &str) -> Result<String> {
    for locale in locale_fallbacks(&config.locale) {
        let path = format!("templates/sdd/{}/{relative_path}", locale);
        if let Some(content) = embedded_template(&path) {
            return expand_template(content, root);
        }
        let full_path = root.join(&path);
        if full_path.exists() {
            let content = fs::read_to_string(&full_path).map_err(|err| {
                anyhow!(t!("sdd.templates.read_failed", path = path, error = err))
            })?;
            return expand_template(&content, root);
        }
    }

    Err(anyhow!(t!("sdd.templates.not_found", path = relative_path)))
}

fn expand_template(raw: &str, root: &Path) -> Result<String> {
    let expanded = expand_regions(raw, |path| load_source_content(path, root))?;
    Ok(expanded.trim_end().to_string())
}

fn load_source_content(path: &str, root: &Path) -> Result<String> {
    if let Some(content) = embedded_template(path) {
        return Ok(content.to_string());
    }

    let resolved = if Path::new(path).is_absolute() {
        Path::new(path).to_path_buf()
    } else {
        root.join(path)
    };

    fs::read_to_string(&resolved).map_err(|err| {
        anyhow!(t!(
            "sdd.templates.read_failed",
            path = resolved.display(),
            error = err
        ))
    })
}

fn embedded_template(path: &str) -> Option<&'static str> {
    match path {
        "templates/sdd/en/agents.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/agents.md"
        ))),
        "templates/sdd/en/agents-root-stub.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/agents-root-stub.md"
        ))),
        "templates/sdd/en/project.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/project.md"
        ))),
        "templates/sdd/en/spec-driven/proposal.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/spec-driven/proposal.md"
        ))),
        "templates/sdd/en/spec-driven/spec.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/spec-driven/spec.md"
        ))),
        "templates/sdd/en/spec-driven/design.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/spec-driven/design.md"
        ))),
        "templates/sdd/en/spec-driven/tasks.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/spec-driven/tasks.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-onboard.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-onboard.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-new-change.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-new-change.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-show.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-show.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-validate.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-validate.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-archive.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-archive.md"
        ))),
        "templates/sdd/en/skills/shared.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/shared.md"
        ))),
        "templates/sdd/zh-Hans/agents.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/agents.md"
        ))),
        "templates/sdd/zh-Hans/agents-root-stub.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/agents-root-stub.md"
        ))),
        "templates/sdd/zh-Hans/project.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/project.md"
        ))),
        "templates/sdd/zh-Hans/spec-driven/proposal.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/spec-driven/proposal.md"
        ))),
        "templates/sdd/zh-Hans/spec-driven/spec.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/spec-driven/spec.md"
        ))),
        "templates/sdd/zh-Hans/spec-driven/design.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/spec-driven/design.md"
        ))),
        "templates/sdd/zh-Hans/spec-driven/tasks.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/spec-driven/tasks.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-onboard.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-onboard.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-new-change.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-new-change.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-show.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-show.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-validate.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-validate.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-archive.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-archive.md"
        ))),
        "templates/sdd/zh-Hans/skills/shared.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/shared.md"
        ))),
        _ => None,
    }
}
