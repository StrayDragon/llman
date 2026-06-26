#!/usr/bin/env python3
"""
Embedding API helper for llman sdd index rebuild.

Reads JSON from stdin: {"texts": [...], "api_url": "...", "api_key": "...", "model": "..."}
Outputs JSON to stdout: {"embeddings": [[f32; dim], ...]}
Errors to stderr.
"""
import json, sys, requests, time

def main():
    input_data = json.load(sys.stdin)
    texts = input_data["texts"]
    api_url = input_data.get("api_url", "").rstrip("/")
    api_key = input_data.get("api_key", "")
    model = input_data.get("model", "bge-m3-mlx-8bit")

    # Normalize API URL
    if "/embeddings" not in api_url:
        api_url = api_url.rstrip("/") + "/embeddings"

    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
    }

    all_embeddings = []
    batch_size = 8  # Safe batch size from experiment

    for i in range(0, len(texts), batch_size):
        batch = texts[i:i+batch_size]
        for attempt in range(3):
            try:
                resp = requests.post(
                    api_url,
                    headers=headers,
                    json={"model": model, "input": batch, "encoding_format": "float"},
                    timeout=60,
                )
                resp.raise_for_status()
                data = resp.json()
                # Sort by index to maintain order
                embeddings = [d["embedding"] for d in sorted(data["data"], key=lambda x: x["index"])]
                all_embeddings.extend(embeddings)
                break
            except Exception as e:
                if attempt < 2:
                    time.sleep(1)
                    continue
                print(f"Error embedding batch {i//batch_size}: {e}", file=sys.stderr)
                sys.exit(1)

    print(json.dumps({"embeddings": all_embeddings}))

if __name__ == "__main__":
    main()
