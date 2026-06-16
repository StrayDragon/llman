pub mod toon_backend;

pub use toon_backend::BACKEND;

use crate::sdd::spec::ir::{DeltaSpecDoc, MainSpecDoc};
use anyhow::Result;

pub trait SpecBackend: Sync + Send {
    /// Parse a main spec from a standalone `.toon` file's contents.
    fn parse_main_spec(&self, content: &str, context: &str) -> Result<MainSpecDoc>;

    /// Parse a main spec with strict TOON validation (catches quoting/syntax errors).
    fn parse_main_spec_strict(&self, content: &str, context: &str) -> Result<MainSpecDoc>;

    /// Parse a delta spec from a standalone `.toon` file's contents.
    fn parse_delta_spec(&self, content: &str, context: &str) -> Result<DeltaSpecDoc>;

    /// Parse a delta spec with strict TOON validation.
    fn parse_delta_spec_strict(&self, content: &str, context: &str) -> Result<DeltaSpecDoc>;

    /// Deterministically dump a main spec payload (no surrounding Markdown fence).
    fn dump_main_spec(&self, doc: &MainSpecDoc) -> Result<String>;

    /// Deterministically dump a delta spec payload (no surrounding Markdown fence).
    fn dump_delta_spec(&self, doc: &DeltaSpecDoc) -> Result<String>;
}
