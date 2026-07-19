use super::config::{SddConfig, locale_fallbacks};
use anyhow::{Context, Result, anyhow};
use minijinja::{Environment, ErrorKind, UndefinedBehavior};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const TEMPLATES_ROOT: &str = "templates/sdd";

pub struct SkillTemplate {
    pub name: &'static str,
    pub content: String,
}

const DEFAULT_SKILL_FILES: &[&str] = &[
    "llman-sdd-explore.md",
    "llman-sdd-propose.md",
    "llman-sdd-apply.md",
    "llman-sdd-verify.md",
    "llman-sdd-quick.md",
    "llman-sdd-specs-compact.md",
    "llman-sdd-archive.md",
    "llman-sdd-graph.md",
    "llman-sdd-apply-cycle.md",
];

const OPTIONAL_SKILL_FILES: &[&str] = &[
    "llman-sdd-new-change.md",
    "llman-sdd-continue.md",
    "llman-sdd-ff.md",
    "llman-sdd-sync.md",
    "llman-sdd-validate.md",
];

const UNIT_FILES: &[&str] = &[
    "skills/sdd-commands.md",
    "skills/validation-hints.md",
    "skills/validation-hints-toon.md",
    "spec/toon-contract.md",
    "skills/structured-protocol.md",
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

fn resolve_optional_skills(config: &SddConfig) -> Vec<&'static str> {
    let Some(ref extra) = config.extra_skills else {
        return Vec::new();
    };
    let enabled: HashSet<&str> = extra.iter().map(|s| s.as_str()).collect();
    OPTIONAL_SKILL_FILES
        .iter()
        .filter(|name| {
            let stem = name.trim_end_matches(".md");
            enabled.contains(stem)
        })
        .copied()
        .collect()
}

pub fn skill_templates(config: &SddConfig, root: &Path) -> Result<Vec<SkillTemplate>> {
    let mut files = Vec::new();
    for name in DEFAULT_SKILL_FILES {
        let content = load_skill_template(config, root, name, "default")?;
        files.push(SkillTemplate { name, content });
    }
    for name in resolve_optional_skills(config) {
        let content = load_skill_template(config, root, name, "optional")?;
        files.push(SkillTemplate { name, content });
    }
    Ok(files)
}

pub fn root_stub_content(config: &SddConfig, root: &Path) -> Result<String> {
    load_template(config, root, "agents-root-stub.md")
}

fn load_skill_template(
    config: &SddConfig,
    root: &Path,
    skill_file: &str,
    skill_set: &str,
) -> Result<String> {
    let units = load_template_units(config, root)?;
    let mut vars = build_template_vars(config);
    vars.insert("skill_set".to_string(), skill_set.to_string());
    load_template_with_context(
        config,
        root,
        &format!("skills/{}", skill_file),
        &units,
        &vars,
    )
}

fn load_template(config: &SddConfig, root: &Path, relative_path: &str) -> Result<String> {
    let units = load_template_units(config, root)?;
    let vars = build_template_vars(config);
    load_template_with_context(config, root, relative_path, &units, &vars)
}

fn build_template_vars(config: &SddConfig) -> BTreeMap<String, String> {
    let mut vars = BTreeMap::new();
    // Add llman CLI version for skill metadata
    vars.insert(
        "llman_version".to_string(),
        env!("CARGO_PKG_VERSION").to_string(),
    );
    let bdd_mode = if config.bdd.is_some() { "on" } else { "off" };
    vars.insert("bdd_mode".to_string(), bdd_mode.to_string());
    // Default skill_set; overridden per-skill in load_skill_template.
    vars.insert("skill_set".to_string(), "default".to_string());

    if let Some(ref bdd) = config.bdd {
        vars.insert("bdd_enabled".to_string(), "true".to_string());
        vars.insert("bdd_framework".to_string(), bdd.framework.clone());
        if let Some(ref dir) = bdd.feature_dir {
            vars.insert("bdd_feature_dir".to_string(), dir.clone());
        }
        vars.insert("bdd_run_command".to_string(), bdd.effective_run_command());
        if let Some(ref lang) = bdd.default_language {
            vars.insert("bdd_default_language".to_string(), lang.clone());
        }
        if let Some(ref prompt) = bdd.verify_prompt {
            vars.insert("bdd_verify_prompt".to_string(), prompt.clone());
        }
    }

    let extras: HashSet<&str> = config
        .extra_skills
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();
    for name in OPTIONAL_SKILL_FILES {
        let stem = name.trim_end_matches(".md");
        let key = format!(
            "extra_skill_{}",
            stem.trim_start_matches("llman-sdd-").replace('-', "_")
        );
        if extras.contains(stem) {
            vars.insert(key, "true".to_string());
        }
    }
    vars
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
    let path = format!("{}/{}/{}", TEMPLATES_ROOT, locale, relative_path);
    for base in candidate_template_roots(root) {
        let full_path = base.join(&path);
        if !full_path.exists() {
            continue;
        }
        let content = fs::read_to_string(&full_path)
            .map_err(|err| anyhow!(t!("sdd.templates.read_failed", path = path, error = err)))?;
        return Ok(Some(content));
    }
    if let Some(content) = embedded_template(&path) {
        return Ok(Some(content.to_string()));
    }
    Ok(None)
}

fn candidate_template_roots(root: &Path) -> Vec<PathBuf> {
    let mut roots = vec![root.to_path_buf()];
    let manifest_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if manifest_root != *root {
        roots.push(manifest_root);
    }
    roots
}

fn render_template(
    raw: &str,
    units: &TemplateUnitRegistry,
    vars: &BTreeMap<String, String>,
) -> Result<String> {
    let mut env = Environment::new();
    env.set_undefined_behavior(UndefinedBehavior::Lenient);

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
        "templates/sdd/en/agents-root-stub.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/agents-root-stub.md"
        ))),

        "templates/sdd/en/skills/llman-sdd-propose.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-propose.md"
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

        "templates/sdd/en/skills/llman-sdd-sync.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-sync.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-validate.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-validate.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-archive.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-archive.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-verify.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-verify.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-quick.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-quick.md"
        ))),
        "templates/sdd/en/skills/llman-sdd-specs-compact.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/skills/llman-sdd-specs-compact.md"
        ))),
        "templates/sdd/en/units/skills/sdd-commands.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/units/skills/sdd-commands.md"
        ))),
        "templates/sdd/en/units/skills/validation-hints.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/units/skills/validation-hints.md"
        ))),
        "templates/sdd/en/units/skills/validation-hints-toon.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/units/skills/validation-hints-toon.md"
        ))),
        "templates/sdd/en/units/spec/toon-contract.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/units/spec/toon-contract.md"
        ))),
        "templates/sdd/en/units/skills/structured-protocol.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/en/units/skills/structured-protocol.md"
        ))),
        "templates/sdd/en/units/workflow/archive-freeze-guidance.md" => {
            Some(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/templates/sdd/en/units/workflow/archive-freeze-guidance.md"
            )))
        }
        "templates/sdd/zh-Hans/agents-root-stub.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/agents-root-stub.md"
        ))),

        "templates/sdd/zh-Hans/skills/llman-sdd-propose.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-propose.md"
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

        "templates/sdd/zh-Hans/skills/llman-sdd-sync.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-sync.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-validate.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-validate.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-archive.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-archive.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-verify.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-verify.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-quick.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-quick.md"
        ))),
        "templates/sdd/zh-Hans/skills/llman-sdd-specs-compact.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/skills/llman-sdd-specs-compact.md"
        ))),
        "templates/sdd/zh-Hans/units/skills/sdd-commands.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/units/skills/sdd-commands.md"
        ))),
        "templates/sdd/zh-Hans/units/skills/validation-hints.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/units/skills/validation-hints.md"
        ))),
        "templates/sdd/zh-Hans/units/skills/validation-hints-toon.md" => {
            Some(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/templates/sdd/zh-Hans/units/skills/validation-hints-toon.md"
            )))
        }
        "templates/sdd/zh-Hans/units/spec/toon-contract.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/units/spec/toon-contract.md"
        ))),
        "templates/sdd/zh-Hans/units/skills/structured-protocol.md" => Some(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/templates/sdd/zh-Hans/units/skills/structured-protocol.md"
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
        assert!(err.to_string().contains("render minijinja template"));
    }

    #[test]
    fn render_undefined_variable_is_empty() {
        let registry = TemplateUnitRegistry::default();
        let result = render_template("{{ projectName }}", &registry, &BTreeMap::new()).unwrap();
        // Lenient mode: undefined variables render as empty string
        assert_eq!(result, "");
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
