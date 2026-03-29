# Prompt Manifest

> **English** · [中文](prompt_zh.md)

A **prompt** artifact stores versioned, reusable prompt templates — the building blocks
for LLM-based reasoning, instruction-following, and structured output generation.

## Minimum Example

```toml
[package]
kind        = "prompt"
namespace   = "myorg"
name        = "summariser"
description = "Summarise a document in bullet points"

[prompt]
input_variables = ["document"]

[prompt.user]
text = "Summarise the following in 5 bullet points:\n\n{{document}}"

[metadata]
tags    = ["prompt", "summarisation"]
license = "MIT"
```

## Full Example

```toml
[package]
kind        = "prompt"
namespace   = "myorg"
name        = "chain-of-thought"
description = "Multi-step reasoning — zero-shot CoT, few-shot CoT, Tree-of-Thought"

[prompt]
template_engine = "jinja2"
input_variables = ["problem", "domain", "style"]

[prompt.system]
text = """
You are an expert problem solver. When given a problem, break it into clear,
numbered reasoning steps before arriving at a final answer.
Think step by step. Show your work.
"""

[prompt.user]
text = """
Problem: {{problem}}
Domain: {{domain}}
Style: {{style | default("zero-shot-cot")}}

Let's work through this step by step:
"""

[[prompt.examples]]
input  = { problem = "Is 37 a prime number?", domain = "mathematics" }
output = """
Step 1: 37 is odd → not divisible by 2.
Step 2: 3+7=10 → not divisible by 3.
Step 3: Doesn't end 0 or 5 → not divisible by 5.
Step 4: √37 ≈ 6.1; checked all primes up to 6.
Answer: 37 is prime. ✓
"""

[[prompt.examples]]
input  = { problem = "Explain why the sky is blue", domain = "physics" }
output = """
Step 1: Sunlight contains all visible wavelengths.
Step 2: Rayleigh scattering scatters shorter (blue) wavelengths more.
Step 3: Our eyes see scattered blue light from all sky directions.
Answer: The sky appears blue because of Rayleigh scattering. ✓
"""

[prompt.output_format]
type        = "markdown"
schema      = "numbered-steps + final-answer"
strict      = false

[prompt.model_hints]
preferred   = ["gpt-4o", "claude-3-5-sonnet", "gemini-1.5-pro"]
temperature = 0.2
max_tokens  = 2048

[metadata]
tags     = ["prompt", "chain-of-thought", "reasoning", "few-shot", "cot"]
homepage = "https://github.com/myorg/prompts"
license  = "MIT"
```

## Field Reference

### `[prompt]`

| Field             | Type     | Required | Description                                              |
|-------------------|----------|----------|----------------------------------------------------------|
| `template_engine` | string   | —        | `jinja2` (default), `handlebars`, `mustache`, `plain`   |
| `input_variables` | string[] | —        | Variable names callers must supply                       |

### `[prompt.system]`

| Field  | Type   | Description                                      |
|--------|--------|--------------------------------------------------|
| `text` | string | System-role content injected before the dialogue |

### `[prompt.user]`

| Field  | Type   | Description                                          |
|--------|--------|------------------------------------------------------|
| `text` | string | User-turn template; use `{{variable}}` placeholders  |

### `[[prompt.examples]]`

| Field    | Type   | Description                    |
|----------|--------|--------------------------------|
| `input`  | object | Key-value map of variable values|
| `output` | string | Expected model output          |

### `[prompt.output_format]`

| Field    | Type    | Description                                     |
|----------|---------|-------------------------------------------------|
| `type`   | string  | `markdown`, `json`, `plain`, `xml`              |
| `schema` | string  | Free-text description of the output structure   |
| `strict` | boolean | If true, non-conforming output should be retried|

### `[prompt.model_hints]`

| Field         | Type     | Description                             |
|---------------|----------|-----------------------------------------|
| `preferred`   | string[] | Ordered list of preferred model IDs     |
| `temperature` | float    | Sampling temperature (0.0–2.0)          |
| `max_tokens`  | integer  | Upper bound on output tokens            |

## Content File (`content.json`)

```json
{
  "schema_version": "1.0",
  "kind": "prompt",
  "template_engine": "jinja2",
  "input_variables": ["problem", "domain"],
  "system": "You are an expert problem solver. Think step by step.",
  "user": "Problem: {{problem}}\nDomain: {{domain}}\n\nLet's think step by step:",
  "examples": [
    {
      "input": { "problem": "Is 37 prime?", "domain": "math" },
      "output": "Step 1: 37 is odd…\nAnswer: 37 is prime. ✓"
    }
  ],
  "output_format": { "type": "markdown" },
  "model_hints": { "preferred": ["gpt-4o", "claude-3-5-sonnet"], "temperature": 0.2 }
}
```

## Standards Compatibility

| Standard                | How to use                                          |
|-------------------------|-----------------------------------------------------|
| OpenAI Chat format      | `system.text` → system role; `user.text` → user role|
| Anthropic Messages API  | Same role mapping as OpenAI                         |
| LangChain PromptTemplate| `input_variables` aligns exactly                    |
| LlamaIndex              | Load via `PromptTemplate(template=prompt.user.text)`|
| DSPy                    | Use as `dspy.ChainOfThought` signature              |

## Prompt Variants

| Style          | Description                                    |
|----------------|------------------------------------------------|
| `zero-shot-cot`| "Let's think step by step" appended to user   |
| `few-shot-cot` | Includes worked `examples` before the question |
| `tot`          | Tree-of-Thought: explores branching paths      |
| `self-ask`     | Agent generates and answers its own sub-questions|
| `react`        | ReAct pattern: Reason + Act alternation        |

## Publishing

```bash
agentverse publish --file prompt.toml
# → Published prompt myorg/chain-of-thought@0.1.0

# Retrieve and render
agentverse get --kind prompt --namespace myorg --name chain-of-thought --format json
```

