pub mod toon_backend;

pub use toon_backend::BACKEND;

use crate::sdd::spec::ir::{DeltaSpecDoc, MainSpecDoc};
use anyhow::Result;

pub trait SpecBackend: Sync + Send {
    /// Parse a main spec payload from the Markdown body (frontmatter already removed).
    fn parse_main_spec(&self, content: &str, context: &str) -> Result<MainSpecDoc>;

    /// Parse a delta spec payload from the full Markdown file content.
    fn parse_delta_spec(&self, content: &str, context: &str) -> Result<DeltaSpecDoc>;

    /// Deterministically dump a main spec payload (no surrounding Markdown fence).
    fn dump_main_spec(&self, doc: &MainSpecDoc) -> Result<String>;

    /// Deterministically dump a delta spec payload (no surrounding Markdown fence).
    fn dump_delta_spec(&self, doc: &DeltaSpecDoc) -> Result<String>;
}
