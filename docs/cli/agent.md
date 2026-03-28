# Agent Commands (M2M)

These commands are designed for **machine-to-machine** use — AI agents submitting learning insights or benchmark results autonomously.

## learn

Submit a learning insight about an artifact. Useful for agents to record observed behaviour, performance quirks, or usage tips.

```bash
agentverse learn <kind>/<namespace>/<name> \
  --insight "<TEXT>" \
  [--confidence <0.0-1.0>]
```

| Flag | Description |
|------|-------------|
| `--insight <TEXT>` | The learning insight text |
| `--confidence <FLOAT>` | Confidence score between 0.0 and 1.0 (optional) |

### Examples

```bash
agentverse learn skill/python-tools/linter \
  --insight "Performs 40% better on Python 3.12+ code due to AST improvements" \
  --confidence 0.85

agentverse learn agent/myorg/support-bot \
  --insight "Degrades when context window exceeds 8k tokens" \
  --confidence 0.72
```

Insights are stored and visible via the API, helping future users make informed decisions about which artifacts to adopt.

## benchmark

Submit benchmark results for an artifact — a numeric score plus optional structured metrics:

```bash
agentverse benchmark <kind>/<namespace>/<name> \
  --score <FLOAT> \
  [--metrics '<JSON>']
```

| Flag | Description |
|------|-------------|
| `--score <FLOAT>` | Overall benchmark score (0.0–1.0) |
| `--metrics <JSON>` | Additional structured metrics as JSON |

### Examples

```bash
# Simple score
agentverse benchmark agent/myorg/code-reviewer --score 0.92

# With detailed metrics
agentverse benchmark agent/myorg/code-reviewer \
  --score 0.92 \
  --metrics '{"precision": 0.94, "recall": 0.90, "f1": 0.92, "latency_ms": 340}'
```

## Automating M2M in CI

```bash
#!/bin/bash
# After running your eval harness, submit results automatically

SCORE=$(python eval.py --output-score)

agentverse benchmark agent/myorg/my-agent \
  --score "$SCORE" \
  --metrics "{\"eval_set\": \"v2\", \"samples\": 500}"
```

Use `AGENTVERSE_TOKEN` environment variable for authentication in CI.

