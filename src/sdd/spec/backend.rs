use crate::sdd::project::config::SpecStyle;
use crate::sdd::spec::ir::{DeltaSpecDoc, MainSpecDoc};
use anyhow::Result;

pub mod ison_backend;
pub mod toon_backend;
pub mod yaml_backend;
pub mod yaml_overlay;

#[derive(Debug, Clone, Copy, Default)]
pub struct DumpOptions {
    pub pretty_ison: bool,
}

pub trait SpecBackend: Sync + Send {
    fn style(&self) -> SpecStyle;

    /// Parse a main spec payload from the Markdown body (frontmatter already removed).
    fn parse_main_spec(&self, content: &str, context: &str) -> Result<MainSpecDoc>;

    /// Parse a delta spec payload from the full Markdown file content.
    fn parse_delta_spec(&self, content: &str, context: &str) -> Result<DeltaSpecDoc>;

    /// Deterministically dump a main spec payload (no surrounding Markdown fence).
    fn dump_main_spec(&self, doc: &MainSpecDoc, options: DumpOptions) -> Result<String>;

    /// Deterministically dump a delta spec payload (no surrounding Markdown fence).
    fn dump_delta_spec(&self, doc: &DeltaSpecDoc, options: DumpOptions) -> Result<String>;
}

pub fn backend_for_style(style: SpecStyle) -> &'static dyn SpecBackend {
    match style {
        SpecStyle::Ison => &ison_backend::BACKEND,
        SpecStyle::Toon => &toon_backend::BACKEND,
        SpecStyle::Yaml => &yaml_backend::BACKEND,
    }
}
