# Workflow Manifest

A **workflow** artifact defines a multi-step, directed-acyclic-graph (DAG) pipeline that
composes AgentVerse skills, agents, and external commands into a reproducible sequence.
Workflows are declarative, version-controlled, and exportable to popular orchestration formats.

## Minimum Example

```toml
[package]
kind        = "workflow"
namespace   = "myorg"
name        = "quick-review"
description = "Run AI code review on a PR"

[workflow]
trigger = "manual"

[[workflow.steps]]
id        = "review"
kind      = "skill"
namespace = "agentverse-ci"
artifact  = "code-reviewer"
version   = ">=0.1.0"
inputs    = { diff = "{{trigger.diff_url}}" }

[metadata]
tags    = ["workflow", "code-review"]
license = "MIT"
```

## Full Example

```toml
[package]
kind        = "workflow"
namespace   = "myorg"
name        = "ci-review-pipeline"
description = "Full CI: code analysis → security → release notes → notify"

[workflow]
trigger     = "github_pr"
timeout     = "30m"
concurrency = { max = 3, cancel_in_progress = false }

# ── Step 1: parallel code review + security scan ──────────────────────────────
[[workflow.steps]]
id        = "code-review"
name      = "AI Code Review"
kind      = "skill"
namespace = "agentverse-ci"
artifact  = "code-reviewer"
version   = ">=0.1.0"
inputs    = { diff = "{{trigger.pr_url}}", rules = ["correctness", "performance"] }
on_error  = "fail"
retry     = { max_attempts = 2, delay = "30s" }
timeout   = "5m"

[[workflow.steps]]
id         = "security-scan"
name       = "Security Scanner"
kind       = "skill"
namespace  = "agentverse-ci"
artifact   = "code-reviewer"
version    = ">=0.1.0"
depends_on = []                        # parallel with code-review
inputs     = { diff = "{{trigger.pr_url}}", rules = ["security"] }
on_error   = "warn"
timeout    = "5m"

# ── Step 2: release notes (depends on code-review) ───────────────────────────
[[workflow.steps]]
id         = "release-notes"
name       = "Draft Release Notes"
kind       = "skill"
namespace  = "agentverse-ci"
artifact   = "release-notes-writer"
version    = ">=0.1.0"
depends_on = ["code-review"]
inputs     = { repo = "{{trigger.repo}}", from_ref = "{{trigger.base_sha}}", to_ref = "{{trigger.head_sha}}" }
on_error   = "continue"

# ── Step 3: notify (depends on all) ──────────────────────────────────────────
[[workflow.steps]]
id         = "notify"
name       = "Post PR Comment"
kind       = "http"
depends_on = ["code-review", "security-scan", "release-notes"]
url        = "{{trigger.comments_url}}"
method     = "POST"
headers    = { Authorization = "Bearer {{env.GITHUB_TOKEN}}" }
body       = { body = "**Review complete**\n\n{{steps.code-review.outputs.summary}}" }
on_error   = "warn"

[workflow.outputs]
review_summary = "{{steps.code-review.outputs.summary}}"
security_score = "{{steps.security-scan.outputs.score}}"
release_draft  = "{{steps.release-notes.outputs.notes}}"

[metadata]
tags     = ["workflow", "ci", "pipeline", "dag", "code-review", "security"]
homepage = "https://github.com/myorg/workflows"
license  = "MIT"
```

## Field Reference

### `[workflow]`

| Field                      | Type    | Description                                               |
|----------------------------|---------|-----------------------------------------------------------|
| `trigger`                  | string  | `github_pr`, `schedule`, `webhook`, `manual`              |
| `timeout`                  | string  | Global workflow timeout (e.g. `30m`, `2h`)                |
| `concurrency.max`          | integer | Max parallel workflow instances                           |
| `concurrency.cancel_in_progress` | bool | Cancel older runs when new one starts                |

### `[[workflow.steps]]`

| Field         | Type     | Required | Description                                             |
|---------------|----------|----------|---------------------------------------------------------|
| `id`          | string   | ✅        | Unique step identifier; used in `depends_on`           |
| `name`        | string   | —        | Human-readable step name                               |
| `kind`        | string   | ✅        | `skill`, `agent`, `shell`, `http`                      |
| `namespace`   | string   | cond.    | Required for `skill`/`agent` kinds                     |
| `artifact`    | string   | cond.    | Artifact name within the namespace                     |
| `version`     | string   | cond.    | SemVer constraint (e.g. `>=0.1.0`)                     |
| `depends_on`  | string[] | —        | Step IDs that must complete first; empty = parallel    |
| `inputs`      | object   | —        | Input bindings; supports `{{trigger.*}}` / `{{steps.*}}`|
| `on_error`    | string   | —        | `fail` (default), `warn`, `continue`, `retry`          |
| `retry`       | object   | —        | `max_attempts` and `delay` for automatic retries       |
| `timeout`     | string   | —        | Per-step timeout (overrides workflow-level)            |

### `[workflow.outputs]`

Expose step outputs as named workflow results using `{{steps.<id>.outputs.<key>}}`.

## Step Kinds

| Kind    | Description                                   |
|---------|-----------------------------------------------|
| `skill` | Invoke an AgentVerse skill artifact           |
| `agent` | Delegate to an AgentVerse agent artifact      |
| `shell` | Run a shell command (requires `command` field)|
| `http`  | Make an HTTP request (requires `url` field)   |

## Template Variables

| Variable                       | Description                          |
|--------------------------------|--------------------------------------|
| `{{trigger.pr_url}}`           | PR URL (github_pr trigger)           |
| `{{trigger.repo}}`             | `owner/repo` slug                    |
| `{{trigger.base_sha}}`         | Base commit SHA                      |
| `{{trigger.head_sha}}`         | Head commit SHA                      |
| `{{steps.<id>.outputs.<key>}}` | Output of a previous step            |
| `{{env.VARIABLE_NAME}}`        | Environment variable                 |

## Export Formats

```bash
# Export to GitHub Actions workflow
agentverse get --kind workflow --namespace myorg --name ci-review-pipeline \
  --format github-actions > .github/workflows/ci-review.yml

# Export to Argo WorkflowTemplate
agentverse get --kind workflow --namespace myorg --name ci-review-pipeline \
  --format argo-workflow > ci-review.argo.yaml

# Export to Prefect flow
agentverse get --kind workflow --namespace myorg --name ci-review-pipeline \
  --format prefect > ci_review_flow.py
```

## Standards Compatibility

| Standard       | Export flag         | Notes                                |
|----------------|---------------------|--------------------------------------|
| GitHub Actions | `github-actions`    | Maps steps to `jobs.<id>.steps`      |
| Argo Workflows | `argo-workflow`     | Maps to `WorkflowTemplate` CRD       |
| Prefect        | `prefect`           | Generates a `@flow` Python function  |
| Apache Airflow | `airflow`           | Generates a `DAG` Python module      |
| LangGraph      | `langgraph`         | Generates a `StateGraph`             |

## Publishing

```bash
agentverse publish --file workflow.toml
# → Published workflow myorg/ci-review-pipeline@0.1.0
```

