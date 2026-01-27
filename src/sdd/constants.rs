pub const LLMANSPEC_DIR_NAME: &str = "llmanspec";
pub const LLMANSPEC_CONFIG_FILE: &str = "config.yaml";

pub struct MarkerPair {
    pub start: &'static str,
    pub end: &'static str,
}

pub const LLMANSPEC_MARKERS: MarkerPair = MarkerPair {
    start: "<!-- LLMANSPEC:START -->",
    end: "<!-- LLMANSPEC:END -->",
};

pub const SPEC_DRIVEN_TEMPLATE_DIR: &str = "templates/spec-driven";
