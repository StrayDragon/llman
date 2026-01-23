use std::collections::HashMap;

pub const LLMANSPEC_DIR_NAME: &str = "llmanspec";

pub struct MarkerPair {
    pub start: &'static str,
    pub end: &'static str,
}

pub const LLMANSPEC_MARKERS: MarkerPair = MarkerPair {
    start: "<!-- LLMANSPEC:START -->",
    end: "<!-- LLMANSPEC:END -->",
};

pub const SPEC_DRIVEN_TEMPLATE_DIR: &str = "templates/spec-driven";

pub fn spec_driven_template_files() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        (
            "proposal.md",
            include_str!("../../templates/sdd/spec-driven/proposal.md"),
        ),
        (
            "spec.md",
            include_str!("../../templates/sdd/spec-driven/spec.md"),
        ),
        (
            "design.md",
            include_str!("../../templates/sdd/spec-driven/design.md"),
        ),
        (
            "tasks.md",
            include_str!("../../templates/sdd/spec-driven/tasks.md"),
        ),
    ])
}

pub fn project_template() -> &'static str {
    include_str!("../../templates/sdd/project.md")
}

pub fn managed_block_template() -> &'static str {
    include_str!("../../templates/sdd/llmanspec-managed-block.md")
}
