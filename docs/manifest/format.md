# Manifest Format

> **English** · [中文](format_zh.md)

The `agentverse.toml` manifest file describes your artifact to the registry. It is used by `agentverse publish` to create or update artifacts.

## Full Example

```toml
# agentverse.toml

# ── Package identity ─────────────────────────────────────────────────────────
[package]
kind        = "skill"           # skill | agent | workflow | soul | prompt
namespace   = "myorg"           # your username or org name
name        = "code-linter"     # unique within kind+namespace
description = "A Python code linter skill with AST-based analysis"

# ── Capabilities ─────────────────────────────────────────────────────────────
[capabilities]
input_modalities  = ["text", "json"]
output_modalities = ["text", "json"]
protocols         = ["mcp", "openai-function"]
permissions       = ["network:read", "fs:read"]
max_tokens        = 4096

# ── Dependencies ─────────────────────────────────────────────────────────────
[dependencies]
"python-tools/ast-parser" = ">=1.0.0"
"myorg/base-linter"       = "^2.1.0"

# ── Metadata ─────────────────────────────────────────────────────────────────
[metadata]
tags     = ["python", "linting", "ast", "automation"]
homepage = "https://github.com/myorg/code-linter"
license  = "MIT"
```

## Section Reference

### `[package]`

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `kind` | string | ✅ | Artifact type: `skill`, `agent`, `workflow`, `soul`, or `prompt` |
| `namespace` | string | ✅ | Owner namespace (your username or org) |
| `name` | string | ✅ | Artifact name (lowercase, hyphens, alphanumeric) |
| `description` | string | — | Short description (shown in search results) |

### `[capabilities]`

Stored in the manifest and used for capability-based discovery.

| Key | Type | Description |
|-----|------|-------------|
| `input_modalities` | string[] | Accepted input types: `text`, `json`, `image`, `audio`, `file` |
| `output_modalities` | string[] | Produced output types |
| `protocols` | string[] | Supported protocols: `mcp`, `openai-function`, `a2a`, etc. |
| `permissions` | string[] | Required permissions: `network:read`, `fs:read`, `fs:write`, etc. |
| `max_tokens` | integer | Maximum token context size |

### `[dependencies]`

Declare other AgentVerse artifacts this artifact depends on.

```toml
[dependencies]
"python-tools/ast-parser" = ">=1.0.0, <2.0.0"
"myorg/base-skill"        = "^1.2.0"
```

Version constraint syntax follows SemVer ranges.

### `[metadata]`

| Key | Type | Description |
|-----|------|-------------|
| `tags` | string[] | Searchable tags |
| `homepage` | string | URL to project homepage or GitHub repo |
| `license` | string | SPDX license identifier (e.g. `MIT`, `Apache-2.0`) |

## Artifact Kinds

| Kind | Use Case |
|------|----------|
| `skill` | Reusable tool or capability (code-review, web-scrape, etc.) |
| `agent` | Autonomous AI agent with a defined persona |
| `workflow` | Multi-step orchestration pipeline or DAG |
| `soul` | Personality / persona configuration for an agent |
| `prompt` | Optimized prompt template or chain-of-thought |

---

## Kind-Specific Sections

Each artifact kind supports an optional kind-named TOML section for richer metadata.

### `[soul]` — Persona & Personality

Add a `[soul]` section when `kind = "soul"`:

```toml
[soul]
tone           = "empathetic"       # empathetic | formal | casual | direct | playful
language_style = "conversational"   # conversational | technical | academic | simple

[soul.persona]
name       = "Alex"
background = "Seasoned life coach specializing in mindfulness and CBT"
greeting   = "Hi, I'm here to listen. What's on your mind today?"

[[soul.values]]
name        = "empathy"
description = "Always acknowledge feelings before offering solutions"

[[soul.values]]
name        = "non-judgment"
description = "Avoid evaluative language; accept the user's perspective as valid"

[[soul.constraints]]
rule    = "no_professional_advice"
message = "For serious concerns, please consult a licensed professional."
```

| Field              | Type     | Description                                         |
|--------------------|----------|-----------------------------------------------------|
| `tone`             | string   | Communication style (`empathetic`, `formal`, etc.)  |
| `language_style`   | string   | Vocabulary register (`conversational`, `technical`) |
| `persona.name`     | string   | Display name for the agent persona                  |
| `persona.greeting` | string   | Opening message shown to users                      |
| `values[].name`    | string   | Core value identifier                               |
| `constraints[].rule` | string | Machine-readable constraint identifier             |
| `constraints[].message` | string | Human-readable constraint explanation          |

> **OpenClaw Soul Agents**: Souls published with `[metadata.openclaw]` are automatically
> compatible with OpenClaw Soul Agent runtimes that consume `soul.tone`, `soul.values`,
> and `soul.constraints` to shape LLM system prompts.

---

### `[prompt]` — Template & Reasoning Chains

Add a `[prompt]` section when `kind = "prompt"`:

```toml
[prompt]
template_engine = "jinja2"                      # jinja2 | handlebars | mustache | plain
input_variables = ["problem", "domain", "style"]

[prompt.system]
text = "You are an expert problem solver. Think step by step."

[prompt.user]
text = "Problem: {{problem}}\nDomain: {{domain}}\n\nLet's think step by step:"

[[prompt.examples]]
input  = { problem = "Is 37 prime?", domain = "math" }
output = "Step 1: Check divisibility…\nAnswer: 37 is prime. ✓"

[prompt.output_format]
type   = "markdown"        # markdown | json | plain | xml
schema = "numbered-steps + final-answer"

[prompt.model_hints]
preferred   = ["gpt-4o", "claude-3-5-sonnet"]
temperature = 0.2
max_tokens  = 2048
```

| Field               | Type     | Description                                              |
|---------------------|----------|----------------------------------------------------------|
| `template_engine`   | string   | `jinja2` (default), `handlebars`, `mustache`, `plain`   |
| `input_variables`   | string[] | Variable names that callers must supply                  |
| `system.text`       | string   | System-role content (LLM system prompt)                  |
| `user.text`         | string   | User-role template (supports `{{variable}}` syntax)      |
| `examples`          | array    | Few-shot input/output pairs                              |
| `output_format.type`| string   | Expected output format                                   |
| `model_hints`       | object   | Preferred models and sampling parameters                 |

**Standards compatibility:** `system`/`user` map directly to OpenAI Chat and Anthropic
Messages API roles. `input_variables` aligns with LangChain `PromptTemplate`.

---

### `[workflow]` — Agent Step-by-Step Orchestration

A workflow defines the **step-by-step orchestration logic that constrains how an Agent
executes**. It is a declarative state machine: `entry` is the first step, `context` is the
typed shared state the agent reads and writes, and `transitions` conditionally route the
agent from one step to the next. Add a `[workflow]` section when `kind = "workflow"`:

```toml
[workflow]
entry   = "triage"          # first step the agent executes

[workflow.context]
pr_url = { type = "string",  required = true }
depth  = { type = "string",  default = "shallow", enum = ["shallow", "deep"] }
score  = { type = "integer", default = 0 }

# decision step — agent reasons and writes to context
[[workflow.steps]]
id          = "triage"
kind        = "decision"
instruction = "If PR > 500 changed lines set depth='deep', else 'shallow'."
writes      = ["depth"]

[[workflow.steps.transitions]]
when = "context.depth == 'deep'"
goto = "full-review"

[[workflow.steps.transitions]]
when = "context.depth == 'shallow'"
goto = "quick-check"

# skill step — invoke an AgentVerse skill
[[workflow.steps]]
id     = "full-review"
kind   = "skill"
use    = "agentverse-ci/code-reviewer@>=0.1.0"
inputs = { diff = "{{context.pr_url}}", rules = ["security", "correctness"] }
writes = ["score", "issues"]

[[workflow.steps.transitions]]
when = "context.score >= 80"
goto = "approve"

[[workflow.steps.transitions]]
goto = "request-changes"    # fallback — no `when` always matches
```

| Field | Type | Description |
|-------|------|-------------|
| `entry` | string | ID of the first step the agent executes |
| `context.*` | object | Typed shared state schema (`type`, `required`, `default`, `enum`) |
| `steps[].id` | string | Unique step identifier; referenced by `transitions.goto` |
| `steps[].kind` | string | `decision`, `skill`, `agent`, `parallel`, `loop` |
| `steps[].use` | string | `namespace/name@version` for `skill`/`agent` kinds |
| `steps[].writes` | string[] | Context keys this step is allowed to modify |
| `steps[].instruction` | string | Natural-language instruction for `decision` steps |
| `transitions[].when` | string | Boolean expression on `context.*`; omit for unconditional |
| `transitions[].goto` | string | Next step ID, or `__end__` to terminate |

**Execution model:** The agent executes one step at a time. After each step it evaluates
transitions in order — the first matching `when` wins. `__end__` terminates the workflow.

**Framework compatibility:** Maps to LangGraph `StateGraph`, CrewAI Flows `@router`,
and OpenAI Swarm routines. Export to Prefect (`--format prefect`) also supported.

---

### `[agent]` — Autonomous AI Agents

Add an `[agent]` section when `kind = "agent"`:

```toml
[agent.soul]
namespace = "agentverse"
name      = "empathetic-counselor"
version   = ">=0.1.0"

[[agent.skills]]
namespace = "agentverse-ci"
name      = "code-reviewer"
version   = ">=0.1.0"
alias     = "review_code"    # exposed as MCP tool name

[[agent.skills]]
namespace = "agentverse-ci"
name      = "release-notes-writer"
version   = ">=0.1.0"
alias     = "write_release_notes"
optional  = true

[agent.protocols]
mcp    = { enabled = true, version = "2024-11-05" }
a2a    = { enabled = true, version = "0.2.5" }
openai = { enabled = true, functions = true }

[agent.permissions]
network = ["read"]
fs      = ["read"]
secrets = []

[agent.memory]
context_window = 128000
long_term      = { enabled = true, backend = "pgvector" }

[agent.model]
preferred   = ["claude-3-5-sonnet", "gpt-4o"]
temperature = 0.2
max_tokens  = 4096
```

| Field                 | Type     | Description                                              |
|-----------------------|----------|----------------------------------------------------------|
| `agent.soul`          | object   | Reference to a `soul` artifact (namespace/name/version)  |
| `agent.skills[].alias`| string   | Tool name exposed via MCP and OpenAI Functions           |
| `agent.skills[].optional` | bool | If true, agent continues without this skill if missing   |
| `agent.protocols`     | object   | Enabled protocols: `mcp`, `a2a`, `openai`                |
| `agent.permissions`   | object   | Least-privilege declarations (`network`, `fs`, `secrets`)|
| `agent.memory`        | object   | Context window size and long-term memory backend         |
| `agent.model`         | object   | Preferred LLMs and sampling defaults                     |

**MCP:** Listed `skills` are auto-exposed as MCP tool definitions.

**A2A:** Agents publish an Agent Card at `/.well-known/agent.json` when `a2a.enabled = true`.

---

## Content File

Alongside `agentverse.toml`, provide `content.json` with the actual artifact content:

```json
{
  "schema_version": "1.0",
  "kind": "skill",
  "system_prompt": "You are a Python code linter...",
  "config": {
    "rules": ["E501", "F401"],
    "max_line_length": 88
  }
}
```

The `kind` field in `content.json` **must match** `[package].kind` in the manifest.
The CLI automatically reads `content.json` from the same directory as the manifest.

## OpenClaw Extension

AgentVerse supports the **OpenClaw** metadata standard for richer skill descriptions. Include it under `[metadata.openclaw]`:

```toml
[metadata]
tags     = ["python", "linting"]
homepage = "https://github.com/myorg/code-linter"
license  = "MIT"

[metadata.openclaw]
name        = "Python Code Linter"
description = "A Python code linter with AST-based analysis"
version     = "1.0.0"
author      = "myorg"

  [[metadata.openclaw.commands]]
  name        = "lint"
  description = "Lint Python source files"

    [[metadata.openclaw.commands.arguments]]
    name        = "files"
    description = "List of Python files to lint"
    required    = true

  [[metadata.openclaw.commands]]
  name        = "check"
  description = "Check a single file and return diagnostics"
```

Skills published with OpenClaw metadata are automatically compatible with the [ClawHub](https://clawhub.dev) skill registry and AI agent tools that use the OpenClaw standard.

## Version Bumping

The `--bump` flag or `default_bump` server config controls versioning:

| Bump | Before | After |
|------|--------|-------|
| `patch` | `1.2.0` | `1.2.1` |
| `minor` | `1.2.0` | `1.3.0` |
| `major` | `1.2.0` | `2.0.0` |

The first publish always starts at `0.1.0`.

## Validation Rules

| Field | Constraint |
|-------|-----------|
| `kind` | Must be one of: `skill`, `agent`, `workflow`, `soul`, `prompt` |
| `namespace` | Lowercase alphanumeric + hyphens; must match your username/org |
| `name` | Lowercase alphanumeric + hyphens; unique within kind + namespace |
| `protocols` | Supported: `mcp`, `openai-function`, `a2a`, `langchain` |
| `permissions` | Supported: `network:read`, `network:write`, `fs:read`, `fs:write` |

