use crate::sdd::spec::backend::SpecBackend;
use crate::sdd::spec::fence::extract_all_code_fences;
use crate::sdd::spec::ir::{DeltaSpecDoc, MainSpecDoc};
use anyhow::{Result, anyhow};

pub struct ToonBackend;

pub static BACKEND: ToonBackend = ToonBackend;

const TOON_LANG: &str = "toon";

impl SpecBackend for ToonBackend {
    fn parse_main_spec(&self, content: &str, context: &str) -> Result<MainSpecDoc> {
        let payload = extract_single_toon_payload(content, context)?;
        let doc: MainSpecDoc = serde_toon::from_str(payload.trim())
            .map_err(|err| anyhow!("{context}: failed to parse TOON payload: {err}"))?;
        if doc.kind.trim() != "llman.sdd.spec" {
            return Err(anyhow!(
                "{context}: spec kind must be `llman.sdd.spec`, got `{}`",
                doc.kind.trim()
            ));
        }
        Ok(doc)
    }

    fn parse_delta_spec(&self, content: &str, context: &str) -> Result<DeltaSpecDoc> {
        let payload = extract_single_toon_payload(content, context)?;
        let doc: DeltaSpecDoc = serde_toon::from_str(payload.trim())
            .map_err(|err| anyhow!("{context}: failed to parse TOON payload: {err}"))?;
        if doc.kind.trim() != "llman.sdd.delta" {
            return Err(anyhow!(
                "{context}: delta kind must be `llman.sdd.delta`, got `{}`",
                doc.kind.trim()
            ));
        }
        Ok(doc)
    }

    fn dump_main_spec(&self, doc: &MainSpecDoc) -> Result<String> {
        serde_toon::to_string(doc).map_err(|err| anyhow!("failed to serialize TOON payload: {err}"))
    }

    fn dump_delta_spec(&self, doc: &DeltaSpecDoc) -> Result<String> {
        serde_toon::to_string(doc).map_err(|err| anyhow!("failed to serialize TOON payload: {err}"))
    }
}

fn extract_single_toon_payload(content: &str, context: &str) -> Result<String> {
    let fences = extract_all_code_fences(content)?;
    let toon_fences: Vec<_> = fences.iter().filter(|f| f.lang == TOON_LANG).collect();

    if toon_fences.is_empty() {
        return Err(anyhow!("{context}: missing ```{TOON_LANG} code block"));
    }
    if toon_fences.len() != 1 {
        return Err(anyhow!(
            "{context}: expected exactly 1 ```{TOON_LANG} code block, got {}",
            toon_fences.len()
        ));
    }

    let payload = toon_fences[0].payload.trim();
    if payload.is_empty() {
        return Err(anyhow!(
            "{context}: empty {TOON_LANG} payload in code block starting at line {}",
            toon_fences[0].start_line
        ));
    }
    Ok(payload.to_string())
}
