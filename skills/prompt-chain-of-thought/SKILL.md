---
name: prompt-chain-of-thought
kind: prompt
description: >
  A battle-tested chain-of-thought prompt template that guides LLMs through
  multi-step reasoning. Supports zero-shot CoT, few-shot CoT, and Tree-of-Thought
  variants. Reuses the OpenAI prompt engineering and Anthropic prompt library standards.
tags: [prompt, chain-of-thought, reasoning, llm, openai, anthropic]
version: "0.1.0"
author: agentverse
license: MIT
metadata:
  openclaw:
    homepage: https://github.com/loonghao/agentverse
    emoji: "💭"
---

# Prompt: Chain-of-Thought Reasoning

A **prompt** artifact stores optimized prompt templates — reusable building blocks for
LLM-based reasoning, instructions, and structured outputs.

## What is a Prompt?

| Field          | Description                                               |
|----------------|-----------------------------------------------------------|
| `system`       | The system-level instruction injected before the dialogue |
| `user`         | Template for the user turn (supports `{{variables}}`)     |
| `examples`     | Few-shot examples (input/output pairs)                    |
| `output_format`| Expected output schema (JSON, Markdown, plain text, etc.) |
| `model_hints`  | Preferred models or parameter settings                    |

## Prompt Manifest (`prompt.toml`)

```toml
[package]
kind        = "prompt"
namespace   = "agentverse"
name        = "chain-of-thought"
description = "Multi-step reasoning prompt — zero-shot, few-shot, and Tree-of-Thought variants"

[prompt]
template_engine = "jinja2"      # jinja2 | handlebars | mustache | plain
input_variables = ["problem", "domain", "style"]

[prompt.system]
text = """
You are an expert problem solver. When given a problem, break it down into clear,
numbered reasoning steps before arriving at a final answer.
Think step by step. Show your work.
"""

[prompt.user]
text = """
Problem: {{problem}}
Domain: {{domain}}
Reasoning style: {{style | default("zero-shot-cot")}}

Let's work through this step by step:
"""

[[prompt.examples]]
input  = { problem = "Is 37 a prime number?", domain = "mathematics" }
output = """
Step 1: Check divisibility by 2. 37 is odd, so not divisible by 2.
Step 2: Check divisibility by 3. 3+7=10, not divisible by 3.
Step 3: Check divisibility by 5. Doesn't end in 0 or 5.
Step 4: Check up to √37 ≈ 6.1. Check 7? 7 > 6.1, stop.
Answer: 37 is prime. ✓
"""

[prompt.output_format]
type   = "markdown"
schema = "numbered-steps + final-answer"

[prompt.model_hints]
preferred    = ["gpt-4o", "claude-3-5-sonnet", "gemini-1.5-pro"]
temperature  = 0.2
max_tokens   = 2048

[metadata]
tags     = ["prompt", "chain-of-thought", "reasoning", "few-shot", "cot"]
homepage = "https://github.com/loonghao/agentverse"
license  = "MIT"
```

## AgentVerse Content (`content.json`)

```json
{
  "schema_version": "1.0",
  "kind": "prompt",
  "template_engine": "jinja2",
  "input_variables": ["problem", "domain", "style"],
  "system": "You are an expert problem solver. Break down problems into clear numbered reasoning steps before your final answer. Think step by step. Show your work.",
  "user": "Problem: {{problem}}\nDomain: {{domain}}\nReasoning style: {{style|default('zero-shot-cot')}}\n\nLet's work through this step by step:",
  "examples": [
    {
      "input": { "problem": "Is 37 a prime number?", "domain": "mathematics" },
      "output": "Step 1: 37 is odd → not divisible by 2.\nStep 2: 3+7=10 → not divisible by 3.\nStep 3: Doesn't end 0/5 → not divisible by 5.\nStep 4: √37≈6.1, checked all primes ≤6.\nAnswer: 37 is prime. ✓"
    }
  ],
  "output_format": { "type": "markdown", "schema": "numbered-steps + final-answer" },
  "model_hints": { "preferred": ["gpt-4o", "claude-3-5-sonnet"], "temperature": 0.2 }
}
```

## Usage

### Publish

```bash
agentverse publish --file prompt.toml
```

### Retrieve and use in code

```bash
agentverse get --kind prompt --namespace agentverse --name chain-of-thought
```

```python
import json, subprocess

result = subprocess.check_output([
    "agentverse", "get",
    "--kind", "prompt",
    "--namespace", "agentverse",
    "--name", "chain-of-thought",
    "--format", "json"
])
prompt = json.loads(result)

# Render template
from jinja2 import Template
user_msg = Template(prompt["content"]["user"]).render(
    problem="Explain why the sky is blue",
    domain="physics"
)
```

## Variants

| Style          | Description                                    |
|----------------|------------------------------------------------|
| `zero-shot-cot`| "Let's think step by step" suffix only        |
| `few-shot-cot` | Include worked examples before the question    |
| `tot`          | Tree-of-Thought: explore multiple paths        |
| `self-ask`     | Agent asks and answers sub-questions itself    |

## Standards Compatibility

| Standard                | Compatible? | Notes                                      |
|-------------------------|-------------|-------------------------------------------|
| OpenAI Chat format      | ✅           | Maps `system`/`user` to chat roles        |
| Anthropic messages API  | ✅           | Same role mapping                         |
| LangChain PromptTemplate| ✅           | `input_variables` aligns exactly           |
| LlamaIndex              | ✅           | Can be loaded as `PromptTemplate`         |
| DSPy                    | ✅           | Use as `dspy.ChainOfThought` signature    |

## Notes

- Variables use `{{double-brace}}` Jinja2 syntax by default.
- Use `--format json` with the CLI to get machine-readable output.
- Combine with a `soul` artifact to add persona on top of reasoning style.

