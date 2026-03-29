# Soul Manifest

> **English** · [中文](soul_zh.md)

A **soul** defines the personality, tone, values, and behavioural constraints of an AI agent.
Souls are composable, versionable, and shareable across the AgentVerse registry.

## Minimum Example

```toml
[package]
kind        = "soul"
namespace   = "myorg"
name        = "developer-buddy"
description = "A friendly, precise developer companion"

[soul]
tone           = "casual"
language_style = "technical"

[metadata]
tags    = ["soul", "developer", "friendly"]
license = "MIT"
```

## Full Example

```toml
[package]
kind        = "soul"
namespace   = "myorg"
name        = "empathetic-counselor"
description = "Warm, empathetic counselor persona for support-oriented agents"

# ── Personality ───────────────────────────────────────────────────────────────
[soul]
tone           = "empathetic"
language_style = "conversational"

[soul.persona]
name       = "Alex"
background = "Seasoned life coach with 10 years of mindfulness and CBT experience"
greeting   = "Hi, I'm here to listen. What's on your mind today?"
avatar_url = "https://example.com/avatars/alex.png"

# ── Core Values ──────────────────────────────────────────────────────────────
[[soul.values]]
name        = "empathy"
description = "Always acknowledge feelings before offering solutions"
priority    = 1

[[soul.values]]
name        = "non-judgment"
description = "Avoid evaluative language; accept the user's perspective as valid"
priority    = 2

[[soul.values]]
name        = "active-listening"
description = "Reflect back what the user says before responding"
priority    = 3

# ── Constraints (hard rules) ─────────────────────────────────────────────────
[[soul.constraints]]
rule    = "no_professional_advice"
message = "I'm not a licensed professional. For serious concerns, please consult a specialist."

[[soul.constraints]]
rule    = "no_absolute_promises"
message = "I'll do my best, but I can't make guarantees."

# ── Composition (merge with other souls) ─────────────────────────────────────
[[soul.extends]]
namespace = "agentverse"
name      = "base-professional"
version   = ">=0.1.0"
priority  = 0             # lower priority = overridden by this soul

[metadata]
tags     = ["soul", "empathy", "counseling", "support", "openclaw"]
homepage = "https://github.com/myorg/souls"
license  = "MIT"

[metadata.openclaw]
emoji   = "🌿"
version = "0.1.0"
```

## Field Reference

### `[soul]`

| Field           | Type   | Required | Description                                                 |
|-----------------|--------|----------|-------------------------------------------------------------|
| `tone`          | string | ✅        | `empathetic`, `formal`, `casual`, `direct`, `playful`       |
| `language_style`| string | —        | `conversational`, `technical`, `academic`, `simple`         |

### `[soul.persona]`

| Field        | Type   | Description                                    |
|--------------|--------|------------------------------------------------|
| `name`       | string | Display name shown to users                    |
| `background` | string | Background story injected into system prompt   |
| `greeting`   | string | Opening message                                |
| `avatar_url` | string | URL to an avatar image                         |

### `[[soul.values]]`

| Field         | Type    | Description                                        |
|---------------|---------|----------------------------------------------------|
| `name`        | string  | Value identifier (used in system prompt injection) |
| `description` | string  | How this value manifests in behaviour              |
| `priority`    | integer | Lower number = higher precedence                   |

### `[[soul.constraints]]`

| Field     | Type   | Description                                        |
|-----------|--------|----------------------------------------------------|
| `rule`    | string | Machine-readable constraint ID                     |
| `message` | string | Human-readable message injected into system prompt |

### `[[soul.extends]]`

| Field       | Type   | Description                                        |
|-------------|--------|----------------------------------------------------|
| `namespace` | string | Namespace of the parent soul                       |
| `name`      | string | Name of the parent soul to inherit from            |
| `version`   | string | SemVer constraint for the parent soul              |
| `priority`  | integer| Merge priority (0 = lowest, overridden by this soul)|

## Content File (`content.json`)

```json
{
  "schema_version": "1.0",
  "kind": "soul",
  "system_prompt": "You are Alex, a warm and empathetic life coach. Listen actively, reflect the user's emotions, and gently guide them toward clarity. Never judge, never rush.",
  "soul": {
    "tone": "empathetic",
    "language_style": "conversational",
    "persona": {
      "name": "Alex",
      "greeting": "Hi, I'm here to listen. What's on your mind today?"
    },
    "values": ["empathy", "non-judgment", "active-listening"],
    "constraints": [
      "never give medical or legal advice",
      "always recommend professional help for serious issues"
    ]
  }
}
```

## OpenClaw Soul Agent Integration

Souls with `[metadata.openclaw]` are consumed by OpenClaw Soul Agent runtimes:

```yaml
# openclaw-config.yaml
soul:
  source: agentverse
  namespace: myorg
  name: empathetic-counselor
  version: "0.1.0"
agent:
  system_prompt: "{{soul.system_prompt}}"
  tone: "{{soul.tone}}"
  constraints: "{{soul.constraints}}"
```

## Publishing

```bash
agentverse publish --file soul.toml
# → Published soul myorg/empathetic-counselor@0.1.0
```

## Composing Souls

Multiple souls can be merged. Later entries override earlier ones at the field level:

```bash
# Merge two souls — persona-override takes priority
agentverse agent compose \
  --soul agentverse/base-professional \
  --soul myorg/empathetic-counselor
```

