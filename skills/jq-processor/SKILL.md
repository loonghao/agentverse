---
name: jq-processor
kind: skill
description: "Transform, filter, and reshape JSON data using jq — the lightweight command-line JSON processor. Ideal for extracting fields from API responses, transforming CI artifact payloads, and scripting data pipelines."
version: "0.1.0"
tags: [json, data, cli, transformation]
license: MIT
metadata:
  openclaw:
    homepage: https://stedolan.github.io/jq
    emoji: "🔧"
    requires:
      bins:
        - jq
    install:
      - kind: shell
        linux: "apt-get install -y jq || snap install jq"
        macos: "brew install jq"
        windows: "winget install jqlang.jq"
---

# jq JSON Processor

Transform and filter JSON data using [jq](https://stedolan.github.io/jq),
the lightweight and flexible command-line JSON processor.

## When to use

- Extracting specific fields from REST API responses in CI scripts
- Reshaping JSON payloads before forwarding to downstream services
- Generating human-readable summaries from structured data
- Validating JSON schema compliance in test pipelines

## Inputs

```json
{
  "input": { "users": [{"name": "Alice", "age": 30}, {"name": "Bob", "age": 25}] },
  "filter": ".users[] | select(.age > 26) | .name",
  "compact": false
}
```

| Field     | Required | Description                                              |
|-----------|----------|----------------------------------------------------------|
| `input`   | ✓        | JSON object or array to process                          |
| `filter`  | ✓        | jq filter expression                                     |
| `compact` | ✗        | Output compact JSON without pretty-printing (default: false) |
| `raw`     | ✗        | Output raw strings instead of JSON (default: false)      |

## Common filters

```bash
# Extract all artifact IDs from a registry response
echo $RESPONSE | jq '[.items[].id]'

# Filter skills by tag
echo $SKILLS | jq '[.[] | select(.tags | contains(["ci"]))]'

# Format a summary table
echo $DATA | jq -r '.[] | "\(.name)\t\(.version)"'

# Merge two JSON objects
jq -n --argjson a "$OBJ_A" --argjson b "$OBJ_B" '$a + $b'
```

## Output

```json
{
  "result": "Alice",
  "elapsed_ms": 2
}
```

