pub(crate) const LLMANSPEC_DIR_NAME: &str = "llmanspec";
pub(crate) const LLMANSPEC_CONFIG_FILE: &str = "config.yaml";

/// File name for SDD spec files (main specs and delta specs).
/// Specs are standalone TOON documents (one `.toon` file per spec), not Markdown
/// files wrapping a fenced TOON block. Single source of truth — never inline the
/// literal elsewhere.
pub(crate) const SPEC_FILE: &str = "spec.toon";

pub(crate) struct MarkerPair {
    pub(crate) start: &'static str,
    pub(crate) end: &'static str,
}

pub(crate) const LLMANSPEC_MARKERS: MarkerPair = MarkerPair {
    start: "<!-- LLMANSPEC:START -->",
    end: "<!-- LLMANSPEC:END -->",
};
