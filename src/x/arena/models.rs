use anyhow::{Context, Result, anyhow};
use inquire::MultiSelect;
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

#[derive(Debug, Deserialize)]
struct ModelInfo {
    id: String,
}

pub fn run_list(json: bool) -> Result<()> {
    let models = fetch_models()?;
    if json {
        println!("{}", serde_json::to_string(&models)?);
    } else {
        for id in models {
            println!("{id}");
        }
    }
    Ok(())
}

pub fn run_pick() -> Result<()> {
    let models = fetch_models()?;
    let picked = MultiSelect::new("Select models to participate:", models).prompt()?;
    println!("{}", serde_json::to_string(&picked)?);
    Ok(())
}

fn fetch_models() -> Result<Vec<String>> {
    let api_key = env::var("OPENAI_API_KEY").unwrap_or_default();
    if api_key.trim().is_empty() {
        return Err(anyhow!("OPENAI_API_KEY is required"));
    }

    let base = openai_api_base()?;
    let url = format!("{}/models", base);

    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .context("request /models")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        return Err(anyhow!("GET /models failed: {status}\n{body}"));
    }

    let parsed: ModelsResponse = resp.json().context("parse /models response")?;
    let mut ids = parsed.data.into_iter().map(|m| m.id).collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    Ok(ids)
}

fn openai_api_base() -> Result<String> {
    let raw = env::var("OPENAI_BASE_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            env::var("OPENAI_API_BASE")
                .ok()
                .filter(|s| !s.trim().is_empty())
        });

    let base = raw.unwrap_or_else(|| "https://api.openai.com".to_string());
    Ok(normalize_openai_api_base(&base))
}

pub fn normalize_openai_api_base(input: &str) -> String {
    let mut base = input.trim().trim_end_matches('/').to_string();
    if base.ends_with("/v1") {
        return base;
    }
    base.push_str("/v1");
    base
}

#[cfg(test)]
mod tests {
    use super::normalize_openai_api_base;

    #[test]
    fn normalize_openai_api_base_adds_v1_when_missing() {
        assert_eq!(
            normalize_openai_api_base("https://api.openai.com"),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn normalize_openai_api_base_keeps_v1() {
        assert_eq!(
            normalize_openai_api_base("https://example.com/v1"),
            "https://example.com/v1"
        );
        assert_eq!(
            normalize_openai_api_base("https://example.com/v1/"),
            "https://example.com/v1"
        );
    }
}
