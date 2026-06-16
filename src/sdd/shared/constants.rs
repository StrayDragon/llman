pub const LLMANSPEC_DIR_NAME: &str = "llmanspec";
pub const LLMANSPEC_CONFIG_FILE: &str = "config.yaml";

/// File name for SDD spec files (main specs and delta specs).
/// Specs are standalone TOON documents (one `.toon` file per spec), not Markdown
/// files wrapping a fenced TOON block. Single source of truth — never inline the
/// literal elsewhere.
pub const SPEC_FILE: &str = "spec.toon";

pub struct MarkerPair {
    pub start: &'static str,
    pub end: &'static str,
}

pub const LLMANSPEC_MARKERS: MarkerPair = MarkerPair {
    start: "<!-- LLMANSPEC:START -->",
    end: "<!-- LLMANSPEC:END -->",
};
