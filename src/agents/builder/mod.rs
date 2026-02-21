use anyhow::{Context, Result, anyhow};
use llm_json::{RepairOptions, loads};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct AgentPresetBuildRequest {
    pub agent_id: String,
    pub available_skill_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentPresetBuildOutput {
    pub description: String,
    pub includes: Vec<String>,
    pub system_prompt_md: String,
}

pub fn builder_output_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["description", "includes", "system_prompt_md"],
        "properties": {
            "description": { "type": "string", "minLength": 1 },
            "includes": {
                "type": "array",
                "items": { "type": "string" },
                "uniqueItems": true
            },
            "system_prompt_md": { "type": "string", "minLength": 1 }
        }
    })
}

pub fn parse_and_validate_builder_output(
    agent_id: &str,
    available_skill_ids: &[String],
    raw_json: &str,
) -> Result<AgentPresetBuildOutput> {
    let value = parse_builder_json_value(raw_json)?;
    validate_builder_output_value(agent_id, available_skill_ids, &value)
}

pub fn validate_builder_output_value(
    agent_id: &str,
    available_skill_ids: &[String],
    value: &Value,
) -> Result<AgentPresetBuildOutput> {
    let schema = builder_output_schema();
    let validator =
        jsonschema::validator_for(&schema).context("compile agent builder output schema")?;
    let value = strip_unknown_builder_fields(value);
    if let Err(errors) = validator.validate(&value) {
        return Err(anyhow!(
            "Agent builder output does not match schema: {}",
            errors
        ));
    }

    let mut output: AgentPresetBuildOutput =
        serde_json::from_value(value).context("deserialize agent builder output")?;

    output.description = output.description.trim().to_string();
    if output.description.is_empty() {
        return Err(anyhow!("Agent builder output description is empty"));
    }

    let allowed: HashSet<&str> = available_skill_ids.iter().map(|s| s.as_str()).collect();
    let mut removed = Vec::new();
    output.includes.retain(|skill_id| {
        if skill_id == agent_id {
            removed.push(skill_id.clone());
            return false;
        }
        if !allowed.contains(skill_id.as_str()) {
            removed.push(skill_id.clone());
            return false;
        }
        true
    });
    output.includes.sort();
    output.includes.dedup();
    if !removed.is_empty() {
        eprintln!(
            "Warning: agent builder returned unknown/invalid includes, skipped: {}",
            removed.join(", ")
        );
    }

    if !output.system_prompt_md.contains("## Requirements") {
        output.system_prompt_md = format!(
            "{}\n\n## Requirements\n\n- \n",
            output.system_prompt_md.trim_end()
        );
    }

    Ok(output)
}

fn strip_unknown_builder_fields(value: &Value) -> Value {
    let Value::Object(map) = value else {
        return value.clone();
    };
    let mut out = serde_json::Map::new();
    for key in ["description", "includes", "system_prompt_md"] {
        if let Some(value) = map.get(key) {
            out.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(out)
}

fn parse_builder_json_value(raw_json: &str) -> Result<Value> {
    match serde_json::from_str::<Value>(raw_json) {
        Ok(value) => Ok(value),
        Err(_) => {
            let repaired = loads(raw_json, &RepairOptions::default())
                .map_err(|e| anyhow!("failed to parse builder JSON: {}", e))?;
            Ok(repaired)
        }
    }
}

#[cfg(feature = "agents-ai")]
pub fn build_with_openai(request: &AgentPresetBuildRequest) -> Result<AgentPresetBuildOutput> {
    use adk_rust::prelude::{Content, Llm, LlmRequest, OpenAIClient, OpenAIConfig};
    use futures::StreamExt;

    let api_key = require_env_var("OPENAI_API_KEY")?;
    let model = env_var_fallback("OPENAI_MODEL", "OPENAI_DEFAULT_MODEL")?;
    let base_url = optional_env_var_fallback("OPENAI_BASE_URL", "OPENAI_API_BASE");

    let client = if let Some(base_url) = base_url {
        OpenAIClient::new(OpenAIConfig::compatible(api_key, base_url, &model))
            .context("create OpenAI client")?
    } else {
        OpenAIClient::new(OpenAIConfig::new(api_key, &model)).context("create OpenAI client")?
    };

    let available_skills = request
        .available_skill_ids
        .iter()
        .map(|s| format!("- {}", s))
        .collect::<Vec<_>>()
        .join("\n");

    let system = Content::new("system").with_text(
        "You are a llman agent preset builder. Output MUST be a single JSON object matching the provided JSON schema. No markdown, no commentary.",
    );
    let user = Content::new("user").with_text(format!(
        "agent_id: {id}\n\navailable_skills:\n{skills}\n\nGenerate:\n- description: short 1-line summary\n- includes: array of skill ids (subset of available_skills, MUST NOT include agent_id)\n- system_prompt_md: markdown system prompt body, MUST include routing/decision logic and MUST end with a '## Requirements' section.\n",
        id = request.agent_id,
        skills = available_skills
    ));

    let schema = builder_output_schema();
    let req = LlmRequest::new(&model, vec![system, user]).with_response_schema(schema);

    let runtime = tokio::runtime::Runtime::new().context("create tokio runtime")?;
    let raw = runtime.block_on(async {
        let mut stream = client.generate_content(req, false).await?;
        let mut out = String::new();
        while let Some(chunk) = stream.next().await {
            let resp = chunk?;
            let Some(content) = resp.content else {
                if resp.turn_complete {
                    break;
                }
                continue;
            };
            for part in content.parts {
                if let Some(text) = part.text() {
                    out.push_str(text);
                }
            }
            if resp.turn_complete {
                break;
            }
        }
        Ok::<_, anyhow::Error>(out)
    })?;

    parse_and_validate_builder_output(&request.agent_id, &request.available_skill_ids, &raw)
}

#[cfg(feature = "agents-ai")]
fn require_env_var(name: &str) -> Result<String> {
    let value = std::env::var(name).unwrap_or_default();
    if value.trim().is_empty() {
        return Err(anyhow!(
            "{name} is required for `llman agents new --ai`",
            name = name
        ));
    }
    Ok(value)
}

#[cfg(feature = "agents-ai")]
fn env_var_fallback(primary: &str, fallback: &str) -> Result<String> {
    let primary_value = std::env::var(primary).unwrap_or_default();
    if !primary_value.trim().is_empty() {
        return Ok(primary_value);
    }
    let fallback_value = std::env::var(fallback).unwrap_or_default();
    if !fallback_value.trim().is_empty() {
        return Ok(fallback_value);
    }
    Err(anyhow!(
        "{primary} is required for `llman agents new --ai` (or set {fallback})",
        primary = primary,
        fallback = fallback
    ))
}

#[cfg(feature = "agents-ai")]
fn optional_env_var_fallback(primary: &str, fallback: &str) -> Option<String> {
    let primary_value = std::env::var(primary).unwrap_or_default();
    if !primary_value.trim().is_empty() {
        return Some(primary_value);
    }
    let fallback_value = std::env::var(fallback).unwrap_or_default();
    if !fallback_value.trim().is_empty() {
        return Some(fallback_value);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_builder_output_accepts_valid_json() {
        let agent_id = "foo";
        let available = vec!["a".to_string(), "b".to_string()];
        let value = serde_json::json!({
            "description": "desc",
            "includes": ["a"],
            "system_prompt_md": "hello\n\n## Requirements\n\n- must\n"
        });
        let output = validate_builder_output_value(agent_id, &available, &value).expect("valid");
        assert_eq!(output.description, "desc");
        assert_eq!(output.includes, vec!["a".to_string()]);
        assert!(output.system_prompt_md.contains("## Requirements"));
    }

    #[test]
    fn validate_builder_output_rejects_missing_fields() {
        let agent_id = "foo";
        let available = vec!["a".to_string()];
        let value = serde_json::json!({"description": "x"});
        let err = validate_builder_output_value(agent_id, &available, &value).expect_err("invalid");
        assert!(err.to_string().contains("schema"));
    }

    #[test]
    fn validate_builder_output_filters_invalid_includes() {
        let agent_id = "foo";
        let available = vec!["a".to_string()];
        let value = serde_json::json!({
            "description": "desc",
            "includes": ["foo", "missing", "a"],
            "system_prompt_md": "hello"
        });
        let output = validate_builder_output_value(agent_id, &available, &value).expect("valid");
        assert_eq!(output.includes, vec!["a".to_string()]);
        assert!(output.system_prompt_md.contains("## Requirements"));
    }

    #[test]
    fn validate_builder_output_ignores_unknown_fields() {
        let agent_id = "foo";
        let available = vec!["a".to_string()];
        let value = serde_json::json!({
            "agent_id": "foo",
            "description": "desc",
            "includes": ["a"],
            "system_prompt_md": "hello\n\n## Requirements\n\n- ok\n"
        });
        let output = validate_builder_output_value(agent_id, &available, &value).expect("valid");
        assert_eq!(output.description, "desc");
        assert_eq!(output.includes, vec!["a".to_string()]);
    }
}
