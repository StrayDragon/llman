use crate::sdd::project::config::SpecStyle;
use crate::sdd::spec::backend::{DumpOptions, SpecBackend};
use crate::sdd::spec::fence::extract_all_code_fences;
use crate::sdd::spec::ir::{DeltaSpecDoc, MainSpecDoc};
use anyhow::{Result, anyhow};

pub struct ToonBackend;

pub static BACKEND: ToonBackend = ToonBackend;

impl SpecBackend for ToonBackend {
    fn style(&self) -> SpecStyle {
        SpecStyle::Toon
    }

    fn parse_main_spec(&self, content: &str, context: &str) -> Result<MainSpecDoc> {
        let payload = extract_single_style_payload(content, self.style(), context)?;
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
        let payload = extract_single_style_payload(content, self.style(), context)?;
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

    fn dump_main_spec(&self, doc: &MainSpecDoc, options: DumpOptions) -> Result<String> {
        if options.pretty_ison {
            return Err(anyhow!(
                "--pretty-ison is only supported for spec_style=ison"
            ));
        }
        serde_toon::to_string(doc).map_err(|err| anyhow!("failed to serialize TOON payload: {err}"))
    }

    fn dump_delta_spec(&self, doc: &DeltaSpecDoc, options: DumpOptions) -> Result<String> {
        if options.pretty_ison {
            return Err(anyhow!(
                "--pretty-ison is only supported for spec_style=ison"
            ));
        }
        serde_toon::to_string(doc).map_err(|err| anyhow!("failed to serialize TOON payload: {err}"))
    }
}

fn extract_single_style_payload(
    content: &str,
    expected: SpecStyle,
    context: &str,
) -> Result<String> {
    let fences = extract_all_code_fences(content)?;
    let supported = ["ison", "toon", "yaml"];
    let style_fences = fences
        .iter()
        .filter(|f| supported.contains(&f.lang.as_str()))
        .collect::<Vec<_>>();

    let expected_lang = expected.as_str();
    if let Some(found) = style_fences
        .iter()
        .find(|f| f.lang.as_str() != expected_lang)
    {
        return Err(anyhow!(
            "{context}: spec style mismatch: expected ```{expected_lang} fence, found ```{} at line {}",
            found.lang,
            found.start_line
        ));
    }

    let expected_fences = style_fences
        .iter()
        .filter(|f| f.lang.as_str() == expected_lang)
        .collect::<Vec<_>>();
    if expected_fences.is_empty() {
        return Err(anyhow!("{context}: missing ```{expected_lang} code block"));
    }
    if expected_fences.len() != 1 {
        return Err(anyhow!(
            "{context}: expected exactly 1 ```{expected_lang} code block, got {}",
            expected_fences.len()
        ));
    }

    let payload = expected_fences[0].payload.trim();
    if payload.is_empty() {
        return Err(anyhow!(
            "{context}: empty {expected_lang} payload in code block starting at line {}",
            expected_fences[0].start_line
        ));
    }
    Ok(payload.to_string())
}
