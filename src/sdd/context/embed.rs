use anyhow::{Context as _, Result};
use std::time::Duration;

const BATCH_SIZE: usize = 8;
const MAX_RETRIES: usize = 3;
const RETRY_DELAY_MS: u64 = 1000;

/// Embed a list of texts using an OpenAI-compatible embeddings API.
///
/// Sends batched POST requests to `{api_host}/embeddings` with retry logic.
/// Returns a flat `Vec<Vec<f32>>` where each inner vector is one embedding.
pub fn embed_texts(
    texts: &[&str],
    api_host: &str,
    api_key: &str,
    model: &str,
) -> Result<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }

    // Normalize: ensure host ends without trailing slash for URL building
    let host = api_host.trim_end_matches('/');
    let embeddings_url = format!("{}/embeddings", host);

    let client = reqwest::blocking::Client::new();
    let mut all_embeddings: Vec<Vec<f32>> = Vec::with_capacity(texts.len());

    for batch in texts.chunks(BATCH_SIZE) {
        let batch_embeddings =
            embed_batch_with_retry(batch, &embeddings_url, api_key, model, &client).with_context(
                || {
                    format!(
                        "Failed to embed batch of {} texts via {}",
                        batch.len(),
                        embeddings_url
                    )
                },
            )?;
        all_embeddings.extend(batch_embeddings);
    }

    Ok(all_embeddings)
}

/// Send a single batch to the embeddings API, with retry.
fn embed_batch_with_retry(
    batch: &[&str],
    url: &str,
    api_key: &str,
    model: &str,
    client: &reqwest::blocking::Client,
) -> Result<Vec<Vec<f32>>> {
    let mut last_error = None;

    for attempt in 0..MAX_RETRIES {
        match embed_batch(batch, url, api_key, model, client) {
            Ok(embeddings) => return Ok(embeddings),
            Err(e) => {
                last_error = Some(e);
                if attempt < MAX_RETRIES - 1 {
                    std::thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Embedding request failed after retries")))
}

/// Send a single batch request and parse the response.
fn embed_batch(
    batch: &[&str],
    url: &str,
    api_key: &str,
    model: &str,
    client: &reqwest::blocking::Client,
) -> Result<Vec<Vec<f32>>> {
    let body = serde_json::json!({
        "model": model,
        "input": batch,
        "encoding_format": "float",
    });

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&body)
        .send()
        .with_context(|| format!("HTTP request to {} failed", url))?;

    let status = response.status();
    if status != reqwest::StatusCode::OK {
        let body_text = response
            .text()
            .unwrap_or_else(|_| "<unreadable>".to_string());
        anyhow::bail!("Embedding API returned HTTP {}: {}", status, body_text);
    }

    let body_str = response
        .text()
        .context("Failed to read embedding API response body")?;
    let data: serde_json::Value = serde_json::from_str(&body_str)
        .context("Failed to parse embedding API response as JSON")?;

    let embeddings_arr = data["data"]
        .as_array()
        .context("Response missing 'data' array")?;

    // Sort by index to maintain order (API may return out of order)
    let mut sorted: Vec<&serde_json::Value> = embeddings_arr.iter().collect();
    sorted.sort_by(|a, b| {
        let ai = a["index"].as_u64().unwrap_or(0);
        let bi = b["index"].as_u64().unwrap_or(0);
        ai.cmp(&bi)
    });

    let embeddings: Vec<Vec<f32>> = sorted
        .iter()
        .map(|entry| {
            entry["embedding"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .map(|x| x.as_f64().unwrap_or(0.0) as f32)
                        .collect()
                })
                .ok_or_else(|| anyhow::anyhow!("Entry missing 'embedding' field"))
        })
        .collect::<Result<Vec<_>>>()?;

    if embeddings.len() != batch.len() {
        anyhow::bail!(
            "Expected {} embeddings, got {}",
            batch.len(),
            embeddings.len()
        );
    }

    Ok(embeddings)
}
