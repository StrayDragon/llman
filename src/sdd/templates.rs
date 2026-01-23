use crate::sdd::constants::{
    LLMANSPEC_MARKERS, managed_block_template, project_template, spec_driven_template_files,
};

pub struct TemplateFile {
    pub name: &'static str,
    pub content: String,
}

pub fn spec_driven_templates() -> Vec<TemplateFile> {
    let mut files: Vec<TemplateFile> = Vec::new();
    for (name, content) in spec_driven_template_files() {
        files.push(TemplateFile {
            name,
            content: content.trim_end().to_string(),
        });
    }
    files.sort_by_key(|f| f.name);
    files
}

pub fn render_project_template(project_name: &str) -> String {
    let base = project_template();
    base.replace("{{projectName}}", project_name)
        .replace("{{description}}", "TODO: Describe project purpose")
        .replace("{{techStack}}", "TODO: List key technologies")
}

pub fn managed_block_content() -> String {
    managed_block_template().trim_end().to_string()
}

pub fn default_agents_file() -> String {
    let block = managed_block_content();
    format!(
        "{}\n{}\n{}\n\n## Project Notes\n\n- Add project-specific guidance here.\n",
        LLMANSPEC_MARKERS.start, block, LLMANSPEC_MARKERS.end
    )
}
