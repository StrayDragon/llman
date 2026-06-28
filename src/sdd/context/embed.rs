use anyhow::{Context as _, Result};
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::{CreateEmbeddingRequestArgs, EmbeddingInput};

/// Embed a list of texts using an OpenAI-compatible embeddings API.
///
/// Uses `async_openai::Client` as the unified HTTP client (shared with the chat /
/// tool-calling path used by the pageindex backend). Per the sdd-context spec
/// (async-openai replaces reqwest), batching and retries are handled by the
/// client's built-in backoff, so this issues a single request for all inputs
/// rather than hand-written per-batch loops.
pub async fn embed_texts(
    texts: &[&str],
    api_host: &str,
    api_key: &str,
    model: &str,
) -> Result<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    let config = OpenAIConfig::new()
        .with_api_base(normalize_base(api_host))
        .with_api_key(api_key);
    let client = Client::with_config(config);

    let request = CreateEmbeddingRequestArgs::default()
        .model(model)
        .input(EmbeddingInput::StringArray(
            texts.iter().map(|s| s.to_string()).collect(),
        ))
        .build()
        .context("Failed to build embedding request")?;

    let response = client
        .embeddings()
        .create(request)
        .await
        .with_context(|| format!("Embedding API request via {} failed", api_host))?;

    // The API may return embeddings out of order; restore input order by index.
    let mut data = response.data;
    data.sort_by_key(|e| e.index);
    let embeddings: Vec<Vec<f32>> = data.into_iter().map(|e| e.embedding).collect();

    if embeddings.len() != texts.len() {
        anyhow::bail!(
            "Expected {} embeddings, got {}",
            texts.len(),
            embeddings.len()
        );
    }

    Ok(embeddings)
}

/// Normalize an API host into an OpenAI-compatible base URL.
///
/// async-openai joins `api_base` with request paths like `/embeddings`, so the
/// base must not end with a trailing slash.
fn normalize_base(api_host: &str) -> String {
    api_host.trim_end_matches('/').to_string()
}
