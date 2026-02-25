use super::config::{SddConfig, locale_fallbacks};
use crate::sdd::shared::constants::LLMANSPEC_MARKERS;
use anyhow::{Context, Result, anyhow};
use minijinja::{Environment, ErrorKind, UndefinedBehavior};
use std::collections::BTreeMap;
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

const SPEC_DRIVEN_FILES: &[&str] = &[
    "explore.md",
    "onboard.md",
    "new.md",
    "continue.md",
    "ff.md",
    "apply.md",
    "verify.md",
    "sync.md",
    "archive.md",
    "bulk-archive.md",
    "future.md",
];

const SKILL_FILES: &[&str] = &[
    "llman-sdd-onboard.md",
    "llman-sdd-new-change.md",
    "llman-sdd-explore.md",
    "llman-sdd-continue.md",
    "llman-sdd-ff.md",
    "llman-sdd-apply.md",
    "llman-sdd-verify.md",
    "llman-sdd-sync.md",
    "llman-sdd-bulk-archive.md",
    "llman-sdd-show.md",
    "llman-sdd-validate.md",
    "llman-sdd-archive.md",
    "llman-sdd-specs-compact.md",
];

const OPSX_COMMAND_IDS: &[&str] = &[
    "explore",
    "onboard",
    "new",
    "continue",
    "ff",
    "apply",
    "verify",
    "sync",
    "archive",
    "bulk-archive",
];

const UNIT_FILES: &[&str] = &[
    "skills/sdd-commands.md",
    "skills/validation-hints.md",
    "skills/structured-protocol.md",
    "skills/future-planning.md",
    "workflow/archive-freeze-guidance.md",
];

#[derive(Default, Debug, Clone)]
struct TemplateUnitRegistry {
    units: BTreeMap<String, String>,
}

impl TemplateUnitRegistry {
    fn register(&mut self, id: &str, content: String) -> Result<()> {
        if self.units.contains_key(id) {
            return Err(anyhow!("duplicate template unit id '{}'", id));
        }
        self.units.insert(id.to_string(), content);
        Ok(())
    }
}

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

pub struct OpsxTemplate {
    pub id: &'static str,
    pub content: String,
}

pub fn opsx_templates(config: &SddConfig, root: &Path) -> Result<Vec<OpsxTemplate>> {
    let mut templates = Vec::new();
    for id in OPSX_COMMAND_IDS {
        let content = load_template(config, root, &format!("spec-driven/{id}.md"))?;
        templates.push(OpsxTemplate { id, content });
    }
    Ok(templates)
}

pub fn render_project_template(
    project_name: &str,
    config: &SddConfig,
    root: &Path,
) -> Result<String> {
    let units = load_template_units(config, root)?;
    let mut vars = BTreeMap::new();
    vars.insert("projectName".to_string(), project_name.to_string());
    vars.insert(
        "description".to_string(),
        "TODO: Describe project purpose".to_string(),
    );
    vars.insert(
        "techStack".to_string(),
        "TODO: List key technologies".to_string(),
    );
    load_template_with_context(config, root, "project.md", &units, &vars)
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
    let units = load_template_units(config, root)?;
    load_template_with_context(config, root, relative_path, &units, &BTreeMap::new())
}

fn load_template_with_context(
    config: &SddConfig,
    root: &Path,
    relative_path: &str,
    units: &TemplateUnitRegistry,
    vars: &BTreeMap<String, String>,
) -> Result<String> {
    for locale in locale_fallbacks(&config.locale) {
        if let Some(raw) = load_locale_resource(root, &locale, relative_path)? {
            return render_template(&raw, units, vars)
                .with_context(|| format!("render template {}", relative_path));
        }
    }

    Err(anyhow!(t!("sdd.templates.not_found", path = relative_path)))
}

fn load_template_units(config: &SddConfig, root: &Path) -> Result<TemplateUnitRegistry> {
    let mut registry = TemplateUnitRegistry::default();

    for locale in locale_fallbacks(&config.locale) {
        for unit_file in UNIT_FILES {
            let id = unit_file.trim_end_matches(".md");
            if registry.units.contains_key(id) {
                continue;
            }
            let relative_path = format!("units/{}", unit_file);
            if let Some(content) = load_locale_resource(root, &locale, &relative_path)? {
                registry.register(id, content)?;
            }
        }
    }

    Ok(registry)
}

fn load_locale_resource(root: &Path, locale: &str, relative_path: &str) -> Result<Option<String>> {
    let path = format!("templates/sdd/{}/{}", locale, relative_path);
    if let Some(content) = embedded_template(&path) {
        return Ok(Some(content.to_string()));
    }
    let full_path = root.join(&path);
    if !full_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&full_path)
        .map_err(|err| anyhow!(t!("sdd.templates.read_failed", path = path, error = err)))?;
    Ok(Some(content))
}

fn render_template(
    raw: &str,
    units: &TemplateUnitRegistry,
    vars: &BTreeMap<String, String>,
) -> Result<String> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Strict);

    let unit_map = units.units.clone();
    env.add_function(
        "unit",
        move |id: String| -> std::result::Result<String, minijinja::Error> {
            unit_map.get(&id).cloned().ok_or_else(|| {
                minijinja::Error::new(
                    ErrorKind::InvalidOperation,
                    format!("missing template unit '{}'", id),
                )
            })
        },
    );

    for (key, value) in vars {
        env.add_global(key.clone(), value.clone());
    }

    env.add_template("sdd_template", raw)
        .context("load minijinja template")?;
    let rendered = env
        .get_template("sdd_template")
        .context("get minijinja template")?
        .render(())
        .context("render minijinja template")?;
    Ok(rendered.trim_end().to_string())
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
        "templates/sdd/en/skills/llman-sdd-onboard.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-onboard.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-new-change.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-new-change.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-explore.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-explore.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-continue.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-continue.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-ff.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-ff.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-apply.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-apply.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-verify.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-verify.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-sync.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-sync.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-bulk-archive.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-bulk-archive.md"
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
        "templates/sdd/en/skills/llman-sdd-specs-compact.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-specs-compact.md"
        ))),
        "templates/sdd/en/spec-driven/explore.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/spec-driven/explore.md"
        ))),
        "templates/sdd/en/spec-driven/onboard.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/spec-driven/onboard.md"
        ))),
        "templates/sdd/en/spec-driven/new.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/spec-driven/new.md"
        ))),
        "templates/sdd/en/spec-driven/continue.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/spec-driven/continue.md"
        ))),
        "templates/sdd/en/spec-driven/ff.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/spec-driven/ff.md"
        ))),
        "templates/sdd/en/spec-driven/apply.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/spec-driven/apply.md"
        ))),
        "templates/sdd/en/spec-driven/verify.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/spec-driven/verify.md"
        ))),
        "templates/sdd/en/spec-driven/sync.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/spec-driven/sync.md"
        ))),
        "templates/sdd/en/spec-driven/archive.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/spec-driven/archive.md"
        ))),
        "templates/sdd/en/spec-driven/bulk-archive.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/spec-driven/bulk-archive.md"
        ))),
        "templates/sdd/en/spec-driven/future.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/spec-driven/future.md"
        ))),
        "templates/sdd/en/skills/shared.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/shared.md"
        ))),
        "templates/sdd/en/units/skills/sdd-commands.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/units/skills/sdd-commands.md"
        ))),
        "templates/sdd/en/units/skills/validation-hints.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/units/skills/validation-hints.md"
        ))),
        "templates/sdd/en/units/skills/structured-protocol.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/units/skills/structured-protocol.md"
        ))),
        "templates/sdd/en/units/skills/future-planning.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/units/skills/future-planning.md"
        ))),
        "templates/sdd/en/units/workflow/archive-freeze-guidance.md" => {
            Some(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/templates/sdd/en/units/workflow/archive-freeze-guidance.md"
            )))
        }
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
        "templates/sdd/zh-Hans/skills/llman-sdd-onboard.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-onboard.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-new-change.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-new-change.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-explore.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-explore.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-continue.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-continue.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-ff.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-ff.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-apply.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-apply.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-verify.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-verify.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-sync.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-sync.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-bulk-archive.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-bulk-archive.md"
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
        "templates/sdd/zh-Hans/skills/llman-sdd-specs-compact.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-specs-compact.md"
        ))),
        "templates/sdd/zh-Hans/spec-driven/explore.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/spec-driven/explore.md"
        ))),
        "templates/sdd/zh-Hans/spec-driven/onboard.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/spec-driven/onboard.md"
        ))),
        "templates/sdd/zh-Hans/spec-driven/new.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/spec-driven/new.md"
        ))),
        "templates/sdd/zh-Hans/spec-driven/continue.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/spec-driven/continue.md"
        ))),
        "templates/sdd/zh-Hans/spec-driven/ff.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/spec-driven/ff.md"
        ))),
        "templates/sdd/zh-Hans/spec-driven/apply.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/spec-driven/apply.md"
        ))),
        "templates/sdd/zh-Hans/spec-driven/verify.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/spec-driven/verify.md"
        ))),
        "templates/sdd/zh-Hans/spec-driven/sync.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/spec-driven/sync.md"
        ))),
        "templates/sdd/zh-Hans/spec-driven/archive.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/spec-driven/archive.md"
        ))),
        "templates/sdd/zh-Hans/spec-driven/bulk-archive.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/spec-driven/bulk-archive.md"
        ))),
        "templates/sdd/zh-Hans/spec-driven/future.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/spec-driven/future.md"
        ))),
        "templates/sdd/zh-Hans/skills/shared.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/shared.md"
        ))),
        "templates/sdd/zh-Hans/units/skills/sdd-commands.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/units/skills/sdd-commands.md"
        ))),
        "templates/sdd/zh-Hans/units/skills/validation-hints.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/units/skills/validation-hints.md"
        ))),
        "templates/sdd/zh-Hans/units/skills/structured-protocol.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/units/skills/structured-protocol.md"
        ))),
        "templates/sdd/zh-Hans/units/skills/future-planning.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/units/skills/future-planning.md"
        ))),
        "templates/sdd/zh-Hans/units/workflow/archive-freeze-guidance.md" => {
            Some(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/templates/sdd/zh-Hans/units/workflow/archive-freeze-guidance.md"
            )))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_unit_registration_fails() {
        let mut registry = TemplateUnitRegistry::default();
        registry
            .register("skills/sdd-commands", "first".to_string())
            .expect("register first");
        let err = registry
            .register("skills/sdd-commands", "second".to_string())
            .expect_err("duplicate id should fail");
        assert!(err.to_string().contains("duplicate template unit id"));
    }

    #[test]
    fn render_fails_on_missing_unit() {
        let registry = TemplateUnitRegistry::default();
        let err = render_template(
            "{{ unit(\"skills/does-not-exist\") }}",
            &registry,
            &BTreeMap::new(),
        )
        .expect_err("missing unit should fail");
        assert!(err.to_string().contains("missing template unit"));
    }

    #[test]
    fn render_fails_on_missing_variable() {
        let registry = TemplateUnitRegistry::default();
        let err =
            render_template("{{ projectName }}", &registry, &BTreeMap::new()).expect_err("fail");
        assert!(err.to_string().contains("projectName"));
    }

    #[test]
    fn render_injects_unit_content() {
        let mut registry = TemplateUnitRegistry::default();
        registry
            .register("skills/sdd-commands", "commands".to_string())
            .expect("register");
        let rendered = render_template(
            "Header\n{{ unit(\"skills/sdd-commands\") }}\nFooter",
            &registry,
            &BTreeMap::new(),
        )
        .expect("render");
        assert_eq!(rendered, "Header\ncommands\nFooter");
    }
}
