use crate::sdd::spec::backend::SpecBackend;
use crate::sdd::spec::fence::extract_all_code_fences;
use crate::sdd::spec::ir::{DeltaSpecDoc, MainSpecDoc};
use anyhow::{Result, anyhow};

pub struct ToonBackend;

pub static BACKEND: ToonBackend = ToonBackend;

const TOON_LANG: &str = "toon";

const TOON_FIX_HINT: &str = "\nPlease check your TOON syntax. Common issues: \
    array length mismatch, missing colons, inconsistent delimiters, \
    or unquoted values containing commas/colons/brackets.";

impl SpecBackend for ToonBackend {
    fn parse_main_spec(&self, content: &str, context: &str) -> Result<MainSpecDoc> {
        let payload = extract_single_toon_payload(content, context)?;
        let doc: MainSpecDoc = toon_format::decode_default(payload.trim())
            .map_err(|err| toon_parse_error(context, &err))?;
        validate_spec_kind(&doc.kind, "llman.sdd.spec", context)?;
        Ok(doc)
    }

    fn parse_main_spec_strict(&self, content: &str, context: &str) -> Result<MainSpecDoc> {
        let payload = extract_single_toon_payload(content, context)?;
        let doc: MainSpecDoc = toon_format::decode_strict(payload.trim())
            .map_err(|err| toon_parse_error(context, &err))?;
        validate_spec_kind(&doc.kind, "llman.sdd.spec", context)?;
        Ok(doc)
    }

    fn parse_delta_spec(&self, content: &str, context: &str) -> Result<DeltaSpecDoc> {
        let payload = extract_single_toon_payload(content, context)?;
        let doc: DeltaSpecDoc = toon_format::decode_default(payload.trim())
            .map_err(|err| toon_parse_error(context, &err))?;
        validate_delta_kind(&doc.kind, context)?;
        Ok(doc)
    }

    fn parse_delta_spec_strict(&self, content: &str, context: &str) -> Result<DeltaSpecDoc> {
        let payload = extract_single_toon_payload(content, context)?;
        let doc: DeltaSpecDoc = toon_format::decode_strict(payload.trim())
            .map_err(|err| toon_parse_error(context, &err))?;
        validate_delta_kind(&doc.kind, context)?;
        Ok(doc)
    }

    fn dump_main_spec(&self, doc: &MainSpecDoc) -> Result<String> {
        let payload = toon_format::encode_default(doc)
            .map_err(|err| anyhow!("failed to serialize TOON payload: {err}"))?;
        validate_roundtrip(&payload, "main spec serialization")?;
        Ok(payload)
    }

    fn dump_delta_spec(&self, doc: &DeltaSpecDoc) -> Result<String> {
        let payload = toon_format::encode_default(doc)
            .map_err(|err| anyhow!("failed to serialize TOON payload: {err}"))?;
        validate_roundtrip(&payload, "delta spec serialization")?;
        Ok(payload)
    }
}

fn validate_spec_kind(kind: &str, expected: &str, context: &str) -> Result<()> {
    if kind.trim() != expected {
        return Err(anyhow!(
            "{context}: spec kind must be `{expected}`, got `{}`",
            kind.trim()
        ));
    }
    Ok(())
}

fn validate_delta_kind(kind: &str, context: &str) -> Result<()> {
    if kind.trim() != "llman.sdd.delta" {
        return Err(anyhow!(
            "{context}: delta kind must be `llman.sdd.delta`, got `{}`",
            kind.trim()
        ));
    }
    Ok(())
}

fn toon_parse_error(context: &str, err: &toon_format::ToonError) -> anyhow::Error {
    anyhow!("{context}: failed to parse TOON payload: {err}{TOON_FIX_HINT}")
}

fn validate_roundtrip(payload: &str, label: &str) -> Result<()> {
    let _: serde_json::Value = toon_format::decode_strict(payload.trim())
        .map_err(|err| anyhow!("{label}: round-trip validation failed: {err}{TOON_FIX_HINT}"))?;
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test case: unquoted comma-containing values cause "Expected 3 tabular row
    /// values, but got 5" in strict parsing. This is the exact spec a downstream
    /// user reported as broken in the official TOON viewer.
    #[test]
    fn strict_parse_rejects_unquoted_commas_in_tabular_rows() {
        let bad_toon = r#"kind: llman.sdd.spec
name: cli-entry
purpose: CLI argument parsing and mode dispatch for xylitol.
requirements[4]{req_id,title,statement}:
  r1,clap-args,System MUST parse CLI arguments via clap derive including prompt/config/project/model/list-models/yolo options.
  r2,auto-mode-detect,System MUST auto-detect mode: --acp flag for ACP, prompt present for print/stdio, no prompt for interactive/TUI.
  r3,list-models,System MUST provide --list-models flag that prints available models from config and exits.
  r4,fake-model-cli,System MUST support --model __fake__ to activate fake provider when dev-fake-provider feature is enabled.
scenarios[4]{req_id,id,given,when,then}:
  r1,happy,"",xylitol --config ./test.yaml "do something" is run,args are parsed with config path set and prompt present
  r2,happy,prompt is provided,CLI starts,print/stdio mode is auto-detected and used
  r2,no-prompt,no prompt is provided and ui-tui feature is enabled,CLI starts,interactive/TUI mode is auto-detected
  r2,acp-flag,--acp flag is provided,CLI starts,ACP mode is activated
  r3,happy,config has models defined,xylitol --list-models is run,model table is printed with aliases and providers
  r4,happy,dev-fake-provider feature is enabled,xylitol --model __fake__ "test" is run,fake provider is used without API keys"#;

        let fence = format!("```toon\n{bad_toon}\n```");
        let result = BACKEND.parse_main_spec_strict(&fence, "test");
        assert!(
            result.is_err(),
            "strict parse should reject unquoted commas in tabular values"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("TOON syntax"),
            "error should include TOON fix hint, got: {err}"
        );
    }

    /// Corrected version: commas and inner quotes properly escaped.
    #[test]
    fn strict_parse_accepts_quoted_commas_in_tabular_rows() {
        let good_toon = r#"kind: llman.sdd.spec
name: cli-entry
purpose: CLI argument parsing and mode dispatch for xylitol.
requirements[4]{req_id,title,statement}:
  r1,clap-args,System MUST parse CLI arguments via clap derive including prompt/config/project/model/list-models/yolo options.
  r2,auto-mode-detect,"System MUST auto-detect mode: --acp flag for ACP, prompt present for print/stdio, no prompt for interactive/TUI."
  r3,list-models,System MUST provide --list-models flag that prints available models from config and exits.
  r4,fake-model-cli,System MUST support --model __fake__ to activate fake provider when dev-fake-provider feature is enabled.
scenarios[6]{req_id,id,given,when,then}:
  r1,happy,"","xylitol --config ./test.yaml \"do something\" is run",args are parsed with config path set and prompt present
  r2,happy,prompt is provided,CLI starts,print/stdio mode is auto-detected and used
  r2,no-prompt,no prompt is provided and ui-tui feature is enabled,CLI starts,interactive/TUI mode is auto-detected
  r2,acp-flag,--acp flag is provided,CLI starts,ACP mode is activated
  r3,happy,config has models defined,xylitol --list-models is run,model table is printed with aliases and providers
  r4,happy,dev-fake-provider feature is enabled,"xylitol --model __fake__ \"test\" is run",fake provider is used without API keys"#;

        let fence = format!("```toon\n{good_toon}\n```");
        let doc = BACKEND
            .parse_main_spec_strict(&fence, "test")
            .expect("strict parse should accept properly quoted TOON");
        assert_eq!(doc.name, "cli-entry");
        assert_eq!(doc.requirements.len(), 4);
        assert_eq!(doc.scenarios.len(), 6);
        assert!(
            doc.requirements[1]
                .statement
                .contains("ACP, prompt present")
        );
    }

    #[test]
    fn roundtrip_main_spec_preserves_data() {
        let good_toon = r#"kind: llman.sdd.spec
name: cli-entry
purpose: CLI argument parsing and mode dispatch for xylitol.
requirements[4]{req_id,title,statement}:
  r1,clap-args,System MUST parse CLI arguments via clap derive including prompt/config/project/model/list-models/yolo options.
  r2,auto-mode-detect,"System MUST auto-detect mode: --acp flag for ACP, prompt present for print/stdio, no prompt for interactive/TUI."
  r3,list-models,System MUST provide --list-models flag that prints available models from config and exits.
  r4,fake-model-cli,System MUST support --model __fake__ to activate fake provider when dev-fake-provider feature is enabled.
scenarios[6]{req_id,id,given,when,then}:
  r1,happy,"","xylitol --config ./test.yaml \"do something\" is run",args are parsed with config path set and prompt present
  r2,happy,prompt is provided,CLI starts,print/stdio mode is auto-detected and used
  r2,no-prompt,no prompt is provided and ui-tui feature is enabled,CLI starts,interactive/TUI mode is auto-detected
  r2,acp-flag,--acp flag is provided,CLI starts,ACP mode is activated
  r3,happy,config has models defined,xylitol --list-models is run,model table is printed with aliases and providers
  r4,happy,dev-fake-provider feature is enabled,"xylitol --model __fake__ \"test\" is run",fake provider is used without API keys"#;

        let fence = format!("```toon\n{good_toon}\n```");
        let doc = BACKEND.parse_main_spec(&fence, "test").unwrap();
        let dumped = BACKEND.dump_main_spec(&doc).unwrap();

        // Re-parse the dumped output to verify round-trip
        let fence2 = format!("```toon\n{dumped}\n```");
        let doc2 = BACKEND
            .parse_main_spec_strict(&fence2, "round-trip")
            .expect("round-tripped TOON should parse strictly");
        assert_eq!(doc, doc2);
    }
}
