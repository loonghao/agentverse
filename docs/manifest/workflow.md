# Workflow Manifest

> **English** · [中文](workflow_zh.md)

A **workflow** artifact defines the **step-by-step orchestration logic that constrains how
an Agent executes**. It is a declarative state machine: each step tells the agent what
action to take, what to write into the shared context, and which conditions determine
the next step. The agent is the executor — the workflow is the script it must follow.

```
Workflow  =  Entry step
           + Shared context (typed state the agent reads/writes)
           + Steps (decision | skill | agent | parallel | loop)
           + Transitions (condition → next step)
           + Terminal state (__end__)
```

## Minimum Example

```toml
[package]
kind        = "workflow"
namespace   = "myorg"
name        = "quick-review"
description = "Agent-driven code review with a single skill step"

[workflow]
entry = "review"            # first step the agent executes

[workflow.context]
pr_url = { type = "string", required = true }
score  = { type = "integer", default = 0 }

[[workflow.steps]]
id     = "review"
kind   = "skill"
use    = "agentverse-ci/code-reviewer@>=0.1.0"
inputs = { diff = "{{context.pr_url}}", rules = ["correctness"] }
writes = ["score"]

[[workflow.steps.transitions]]
goto = "__end__"            # terminal — agent stops here

[metadata]
tags    = ["workflow", "code-review"]
license = "MIT"
```

## Full Example — PR Review with Triage, Branching & Retry

```toml
[package]
kind        = "workflow"
namespace   = "myorg"
name        = "pr-review-flow"
description = "Agent-driven PR review: triage → branch by depth → approve or request-changes"

# ── Workflow entry & global settings ─────────────────────────────────────────
[workflow]
entry   = "triage"          # first step the agent must execute
timeout = "30m"

# ── Shared context schema (typed state the agent reads/writes) ────────────────
[workflow.context]
pr_url   = { type = "string",  required = true,  description = "PR URL passed at invocation" }
depth    = { type = "string",  default = "shallow", enum = ["shallow", "deep"] }
score    = { type = "integer", default = 0 }
issues   = { type = "array",   default = [] }
approved = { type = "boolean", default = false }

# ── Step 1: Triage — agent DECIDES review depth ───────────────────────────────
[[workflow.steps]]
id          = "triage"
name        = "Triage PR scope"
kind        = "decision"        # agent reasons and writes to context
instruction = """
Fetch the PR at {{context.pr_url}} and count the total changed lines.
If changed lines > 500 or the PR touches security-sensitive paths, set depth = "deep".
Otherwise set depth = "shallow".
"""
writes = ["depth"]              # declares which context keys this step may modify

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
goto = "approve"              # unconditional — always proceeds to approve

# ── Step 3a: Approve ─────────────────────────────────────────────────────────
[[workflow.steps]]
id     = "approve"
name   = "Post approval"
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
tags     = ["workflow", "agent", "orchestration", "state-machine", "branching"]
homepage = "https://github.com/myorg/workflows"
license  = "MIT"
```

## Field Reference

### `[workflow]`

| Field     | Type    | Required | Description                                               |
|-----------|---------|----------|-----------------------------------------------------------|
| `entry`   | string  | ✅        | ID of the first step the agent executes                   |
| `timeout` | string  | —        | Global execution timeout (e.g. `30m`, `2h`)               |

### `[workflow.context]`

Defines the **shared typed state** that all steps read from and write to.
The agent runtime initialises context from the invocation payload and updates it after each step.

```toml
[workflow.context]
pr_url = { type = "string",  required = true }
score  = { type = "integer", default = 0 }
depth  = { type = "string",  default = "shallow", enum = ["shallow", "deep"] }
issues = { type = "array",   default = [] }
```

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | `string`, `integer`, `boolean`, `array`, `object` |
| `required` | boolean | If true, must be supplied at invocation |
| `default` | any | Value when not supplied |
| `enum` | array | Allowed values for `string` type |
| `description` | string | Documentation hint |

### `[[workflow.steps]]`

| Field         | Type     | Required | Description                                                  |
|---------------|----------|----------|--------------------------------------------------------------|
| `id`          | string   | ✅        | Unique step identifier; referenced by `transitions.goto`    |
| `name`        | string   | —        | Human-readable step name                                    |
| `kind`        | string   | ✅        | `decision`, `skill`, `agent`, `parallel`, `loop`            |
| `use`         | string   | cond.    | `namespace/name@version` — required for `skill`/`agent`     |
| `inputs`      | object   | —        | Key-value map; supports `{{context.*}}` and `{{env.*}}`     |
| `writes`      | string[] | —        | Context keys this step is allowed to modify                  |
| `instruction` | string   | cond.    | Natural-language instruction for `decision` kind             |
| `on_error`    | string   | —        | `fail` (default), `warn`, `continue`, `retry`               |
| `retry`       | object   | —        | `max_attempts` and `backoff` for `on_error = "retry"`       |
| `timeout`     | string   | —        | Per-step timeout; overrides workflow-level                   |

### `[[workflow.steps.transitions]]`

Transitions control where the agent goes after a step completes.
They are evaluated **in order**; the first matching condition wins.

| Field  | Type   | Required | Description                                                    |
|--------|--------|----------|----------------------------------------------------------------|
| `when` | string | —        | Boolean expression on `context.*`; omit for unconditional jump |
| `goto` | string | ✅        | ID of the next step, or `__end__` to terminate the workflow    |

**Expression syntax:** `context.score >= 80`, `context.depth == 'deep'`, `!context.approved`

## Step Kinds

### `decision` — Agent reasons and updates context

The agent receives the `instruction`, reads the current context, and writes results
back to the keys declared in `writes`. No external skill is called.

```toml
[[workflow.steps]]
id          = "triage"
kind        = "decision"
instruction = "If context.pr_url touches >500 lines, set depth = 'deep'."
writes      = ["depth"]

[[workflow.steps.transitions]]
when = "context.depth == 'deep'"
goto = "full-review"

[[workflow.steps.transitions]]
when = "context.depth == 'shallow'"
goto = "quick-check"
```

### `skill` — Invoke an AgentVerse skill

```toml
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
goto = "request-changes"   # fallback — no `when` = always matches
```

### `parallel` — Fan-out to multiple branches simultaneously

```toml
[[workflow.steps]]
id       = "scan-all"
kind     = "parallel"
branches = ["security-scan", "lint-check", "type-check"]
join     = "all"           # all | any | first

[[workflow.steps.transitions]]
goto = "aggregate"
```

### `loop` — Repeat until condition is satisfied

```toml
[[workflow.steps]]
id             = "fix-loop"
kind           = "loop"
body           = "run-tests"   # step to repeat
until          = "context.tests_pass == true"
max_iterations = 5

[[workflow.steps.transitions]]
when = "loop.done"
goto = "deploy"

[[workflow.steps.transitions]]
when = "loop.failed"
goto = "notify-failure"
```

### `agent` — Delegate to another AgentVerse agent

```toml
[[workflow.steps]]
id     = "security-agent"
kind   = "agent"
use    = "myorg/security-auditor@>=1.0.0"
inputs = { repo = "{{context.pr_url}}" }
writes = ["security_score"]

[[workflow.steps.transitions]]
goto = "decide-merge"
```

## Context Template Syntax

Inside `inputs`, `instruction`, and `transitions.when`, use `{{...}}` to reference values:

| Expression | Description |
|------------|-------------|
| `{{context.pr_url}}` | Read a context key |
| `{{env.GITHUB_TOKEN}}` | Read an environment variable |
| `context.score >= 80` | Numeric comparison (in `when`) |
| `context.depth == 'deep'` | String equality (in `when`) |
| `!context.approved` | Boolean negation (in `when`) |
| `loop.done` / `loop.failed` | Loop termination signals |

## State Machine Execution Model

```
┌──────────┐   when depth='deep'   ┌─────────────┐
│  triage  │ ─────────────────────▶│ full-review  │──┐ score>=80 ──▶ approve ──▶ __end__
│(decision)│                       └─────────────┘  └─ score<80  ──▶ request-changes ──▶ __end__
│          │ ─────────────────────▶│ quick-check │──────────────────▶ approve ──▶ __end__
└──────────┘   when depth='shallow' └─────────────┘
```

- The **agent** owns execution — it reads the step, performs the action, updates context, evaluates transitions.
- **`writes`** is enforced: a step cannot modify context keys it hasn't declared.
- Transitions are evaluated sequentially; the first matching `when` wins.
- Reaching `__end__` or exhausting transitions terminates the workflow.

## Standards Compatibility

| Framework   | Mapping                                                        |
|-------------|----------------------------------------------------------------|
| LangGraph   | Steps → nodes; transitions → conditional edges; context → state|
| CrewAI Flows| Steps → `@listen`/`@router` handlers; context → `self.state`  |
| OpenAI Swarm| Steps → routines; `agent` steps → handoffs                    |
| Prefect     | Steps → tasks; `parallel` → `asyncio.gather`; loop → retry    |

## Publishing

```bash
agentverse publish --file workflow.toml
# → Published workflow myorg/pr-review-flow@0.1.0

# Run with initial context
agentverse run --kind workflow --namespace myorg --name pr-review-flow \
  --context pr_url=https://github.com/org/repo/pull/42
```

