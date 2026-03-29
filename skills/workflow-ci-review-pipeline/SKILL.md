---
name: workflow-ci-review-pipeline
kind: workflow
description: >
  An agent-driven PR review workflow that constrains step-by-step execution:
  the agent triages PR scope, branches by review depth, invokes code-reviewer skills,
  and terminates with approve or request-changes — all guided by a declarative state machine.
tags: [workflow, agent, orchestration, state-machine, branching, code-review]
version: "0.1.0"
author: agentverse
license: MIT
metadata:
  openclaw:
    homepage: https://github.com/loonghao/agentverse
    emoji: "🔄"
---

# Workflow: CI Review Pipeline

A **workflow** artifact defines the **step-by-step orchestration logic that constrains
how an Agent executes**. It is a declarative state machine: each step tells the agent
what to do, what to write into the shared context, and which condition determines
the next step. The agent is the executor — the workflow is the script it must follow.

## Core Concepts

| Concept | Description |
|---------|-------------|
| `entry` | The first step the agent executes |
| `context` | Typed shared state — agents read from and write to it across all steps |
| `decision` | A step where the agent reasons and sets context keys (no external skill) |
| `skill` | A step that invokes an AgentVerse skill and writes outputs to context |
| `transitions` | Conditional routing rules — evaluated in order after each step |
| `writes` | Declares which context keys a step is allowed to modify |
| `__end__` | Special goto target that terminates the workflow |

## Workflow Manifest (`workflow.toml`)

```toml
[package]
kind        = "workflow"
namespace   = "agentverse"
name        = "ci-review-pipeline"
description = "Agent-driven PR review: triage → branch by depth → approve or request-changes"

# ── Entry & global settings ───────────────────────────────────────────────────
[workflow]
entry   = "triage"
timeout = "30m"

# ── Shared context (typed state the agent reads/writes) ───────────────────────
[workflow.context]
pr_url   = { type = "string",  required = true }
depth    = { type = "string",  default = "shallow", enum = ["shallow", "deep"] }
score    = { type = "integer", default = 0 }
issues   = { type = "array",   default = [] }
approved = { type = "boolean", default = false }

# ── Step 1: Triage — agent DECIDES review depth ───────────────────────────────
[[workflow.steps]]
id          = "triage"
name        = "Triage PR scope"
kind        = "decision"
instruction = """
Fetch the PR at {{context.pr_url}} and count the total changed lines.
If > 500 lines or security-sensitive paths are touched, set depth = "deep".
Otherwise set depth = "shallow".
"""
writes = ["depth"]

[[workflow.steps.transitions]]
when = "context.depth == 'deep'"
goto = "full-review"

[[workflow.steps.transitions]]
when = "context.depth == 'shallow'"
goto = "quick-check"

# ── Step 2a: Full review (deep path) ─────────────────────────────────────────
[[workflow.steps]]
id       = "full-review"
name     = "Full code review"
kind     = "skill"
use      = "agentverse-ci/code-reviewer@>=0.1.0"
inputs   = { diff = "{{context.pr_url}}", rules = ["security", "correctness", "performance", "style"] }
writes   = ["score", "issues"]
on_error = "retry"
retry    = { max_attempts = 2, backoff = "30s" }

[[workflow.steps.transitions]]
when = "context.score >= 80"
goto = "approve"

[[workflow.steps.transitions]]
when = "context.score < 80"
goto = "request-changes"

# ── Step 2b: Quick check (shallow path) ──────────────────────────────────────
[[workflow.steps]]
id     = "quick-check"
name   = "Style-only check"
kind   = "skill"
use    = "agentverse-ci/code-reviewer@>=0.1.0"
inputs = { diff = "{{context.pr_url}}", rules = ["style"] }
writes = ["score", "issues"]

[[workflow.steps.transitions]]
goto = "approve"

# ── Step 3a: Approve ─────────────────────────────────────────────────────────
[[workflow.steps]]
id     = "approve"
name   = "Post approval comment"
kind   = "skill"
use    = "agentverse-ci/pr-commenter@>=0.1.0"
inputs = { message = "✅ Review passed (score: {{context.score}})", approve = true }
writes = ["approved"]

[[workflow.steps.transitions]]
goto = "__end__"

# ── Step 3b: Request changes ──────────────────────────────────────────────────
[[workflow.steps]]
id     = "request-changes"
name   = "Post change requests"
kind   = "skill"
use    = "agentverse-ci/pr-commenter@>=0.1.0"
inputs = { issues = "{{context.issues}}", score = "{{context.score}}", approve = false }

[[workflow.steps.transitions]]
goto = "__end__"

[metadata]
tags     = ["workflow", "agent", "state-machine", "branching", "code-review"]
homepage = "https://github.com/loonghao/agentverse"
license  = "MIT"
```

## State Machine Diagram

```
┌──────────┐  depth='deep'   ┌─────────────┐  score>=80  ┌─────────┐
│  triage  │────────────────▶│ full-review  │────────────▶│ approve │──▶ __end__
│(decision)│                 │   (skill)    │  score<80   └─────────┘
│          │                 └─────────────┘────────────▶┌──────────────────┐
│          │  depth='shallow' ┌─────────────┐             │ request-changes  │──▶ __end__
│          │────────────────▶│ quick-check  │────────────▶└──────────────────┘
└──────────┘                 │   (skill)    │ (unconditional)
                             └─────────────┘
```

The agent follows this state machine **step by step**:
1. **`triage`** — agent reasons about PR size and writes `depth` to context
2. Transitions route to **`full-review`** or **`quick-check`** based on `depth`
3. Each review step writes `score` and `issues` to context
4. Transitions route to **`approve`** or **`request-changes`** based on `score`
5. Terminal steps go to **`__end__`**

## AgentVerse Content (`content.json`)

```json
{
  "schema_version": "1.0",
  "kind": "workflow",
  "entry": "triage",
  "context": {
    "pr_url":   { "type": "string",  "required": true },
    "depth":    { "type": "string",  "default": "shallow", "enum": ["shallow","deep"] },
    "score":    { "type": "integer", "default": 0 },
    "issues":   { "type": "array",   "default": [] },
    "approved": { "type": "boolean", "default": false }
  },
  "steps": [
    {
      "id": "triage", "kind": "decision",
      "instruction": "Count changed lines in {{context.pr_url}}. Set depth='deep' if >500.",
      "writes": ["depth"],
      "transitions": [
        { "when": "context.depth == 'deep'",    "goto": "full-review"  },
        { "when": "context.depth == 'shallow'", "goto": "quick-check"  }
      ]
    },
    {
      "id": "full-review", "kind": "skill",
      "use": "agentverse-ci/code-reviewer@>=0.1.0",
      "inputs": { "diff": "{{context.pr_url}}", "rules": ["security","correctness","performance","style"] },
      "writes": ["score","issues"],
      "on_error": "retry", "retry": { "max_attempts": 2, "backoff": "30s" },
      "transitions": [
        { "when": "context.score >= 80", "goto": "approve"          },
        { "when": "context.score < 80",  "goto": "request-changes"  }
      ]
    },
    {
      "id": "quick-check", "kind": "skill",
      "use": "agentverse-ci/code-reviewer@>=0.1.0",
      "inputs": { "diff": "{{context.pr_url}}", "rules": ["style"] },
      "writes": ["score","issues"],
      "transitions": [{ "goto": "approve" }]
    },
    {
      "id": "approve", "kind": "skill",
      "use": "agentverse-ci/pr-commenter@>=0.1.0",
      "inputs": { "message": "✅ Review passed (score: {{context.score}})", "approve": true },
      "writes": ["approved"],
      "transitions": [{ "goto": "__end__" }]
    },
    {
      "id": "request-changes", "kind": "skill",
      "use": "agentverse-ci/pr-commenter@>=0.1.0",
      "inputs": { "issues": "{{context.issues}}", "score": "{{context.score}}", "approve": false },
      "transitions": [{ "goto": "__end__" }]
    }
  ]
}
```

## Publish and Run

```bash
# Publish the workflow
agentverse publish --file workflow.toml
# → Published workflow agentverse/ci-review-pipeline@0.1.0

# Run with initial context (agent executes step by step)
agentverse run --kind workflow --namespace agentverse --name ci-review-pipeline \
  --context pr_url=https://github.com/org/repo/pull/42
```

## Framework Compatibility

| Framework   | Mapping                                                        |
|-------------|----------------------------------------------------------------|
| LangGraph   | Steps → nodes; transitions → conditional edges; context → state|
| CrewAI Flows| Steps → `@listen`/`@router`; context → `self.state`           |
| OpenAI Swarm| Steps → routines; `agent` steps → handoffs                    |
| Prefect     | Steps → tasks; `parallel` → `asyncio.gather`; loop → retry    |

## Notes

- **Context is the single source of truth** — all steps read from and write to it.
- **`writes` is enforced** — a step cannot touch context keys it hasn't declared.
- **`decision` steps require no external tool** — the agent itself reasons and writes.
- **Transitions are ordered** — the first matching `when` wins; omit `when` for a catch-all.
- Combine with a `soul` artifact to give the agent a consistent persona across all steps.

