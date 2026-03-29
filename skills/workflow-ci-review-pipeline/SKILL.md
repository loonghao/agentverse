---
name: workflow-ci-review-pipeline
kind: workflow
description: >
  An end-to-end CI review workflow that orchestrates code analysis, security scanning,
  test execution, and release-note generation as a composable DAG pipeline.
  Reuses AgentVerse skills as steps; compatible with GitHub Actions, Argo Workflows,
  and Prefect DAG standards.
tags: [workflow, ci, pipeline, dag, code-review, security, release]
version: "0.1.0"
author: agentverse
license: MIT
metadata:
  openclaw:
    homepage: https://github.com/loonghao/agentverse
    emoji: "🔄"
---

# Workflow: CI Review Pipeline

A **workflow** artifact defines a multi-step, directed-acyclic-graph (DAG) pipeline
that composes AgentVerse skills, agents, and prompts into a reproducible sequence.

## What is a Workflow?

| Concept    | Description                                                  |
|------------|--------------------------------------------------------------|
| `step`     | A single unit of work (runs a skill, agent, or shell command)|
| `dag`      | Defines dependency ordering between steps                    |
| `trigger`  | When the workflow fires (event, schedule, manual, webhook)   |
| `context`  | Shared data passed between steps via `outputs` / `inputs`   |
| `on_error` | Step-level and workflow-level error handling strategies      |

## Workflow Manifest (`workflow.toml`)

```toml
[package]
kind        = "workflow"
namespace   = "agentverse"
name        = "ci-review-pipeline"
description = "Full CI review: code analysis → security → tests → release notes"

[workflow]
trigger = "github_pr"    # github_pr | schedule | webhook | manual

[[workflow.steps]]
id          = "code-review"
name        = "AI Code Review"
kind        = "skill"
namespace   = "agentverse-ci"
artifact    = "code-reviewer"
version     = ">=0.1.0"
inputs      = { diff = "{{trigger.pr_url}}", rules = ["security", "correctness", "performance"] }
on_error    = "fail"

[[workflow.steps]]
id          = "security-scan"
name        = "Security Scanner"
kind        = "skill"
namespace   = "agentverse-ci"
artifact    = "code-reviewer"
version     = ">=0.1.0"
depends_on  = []                  # runs in parallel with code-review
inputs      = { diff = "{{trigger.pr_url}}", rules = ["security"] }
on_error    = "warn"

[[workflow.steps]]
id          = "release-notes"
name        = "Draft Release Notes"
kind        = "skill"
namespace   = "agentverse-ci"
artifact    = "release-notes-writer"
version     = ">=0.1.0"
depends_on  = ["code-review"]
inputs      = { repo = "{{trigger.repo}}", from_ref = "{{trigger.base_sha}}", to_ref = "{{trigger.head_sha}}" }
on_error    = "continue"

[workflow.outputs]
review_summary = "{{steps.code-review.outputs.summary}}"
release_draft  = "{{steps.release-notes.outputs.notes}}"

[metadata]
tags     = ["workflow", "ci", "pipeline", "dag", "code-review"]
homepage = "https://github.com/loonghao/agentverse"
license  = "MIT"
```

## AgentVerse Content (`content.json`)

```json
{
  "schema_version": "1.0",
  "kind": "workflow",
  "trigger": "github_pr",
  "steps": [
    {
      "id": "code-review",
      "name": "AI Code Review",
      "skill": "agentverse-ci/code-reviewer@>=0.1.0",
      "inputs": { "diff": "{{trigger.pr_url}}", "rules": ["security", "correctness"] },
      "on_error": "fail"
    },
    {
      "id": "security-scan",
      "name": "Security Scanner",
      "skill": "agentverse-ci/code-reviewer@>=0.1.0",
      "depends_on": [],
      "inputs": { "diff": "{{trigger.pr_url}}", "rules": ["security"] },
      "on_error": "warn"
    },
    {
      "id": "release-notes",
      "name": "Draft Release Notes",
      "skill": "agentverse-ci/release-notes-writer@>=0.1.0",
      "depends_on": ["code-review"],
      "inputs": { "repo": "{{trigger.repo}}", "from_ref": "{{trigger.base_sha}}", "to_ref": "{{trigger.head_sha}}" },
      "on_error": "continue"
    }
  ],
  "outputs": {
    "review_summary": "{{steps.code-review.outputs.summary}}",
    "release_draft": "{{steps.release-notes.outputs.notes}}"
  }
}
```

## GitHub Actions Integration

```yaml
on:
  pull_request:
    types: [opened, synchronize]

jobs:
  agentverse-ci:
    runs-on: ubuntu-latest
    steps:
      - name: Run CI Review Pipeline
        uses: agentverse/run-workflow@v1
        with:
          workflow: agentverse/ci-review-pipeline
          trigger_context: |
            pr_url: ${{ github.event.pull_request.url }}
            repo: ${{ github.repository }}
            base_sha: ${{ github.event.pull_request.base.sha }}
            head_sha: ${{ github.event.pull_request.head.sha }}
        env:
          AGENTVERSE_TOKEN: ${{ secrets.AGENTVERSE_TOKEN }}
```

## Argo Workflows Compatibility

```yaml
# Export as Argo WorkflowTemplate
agentverse get --kind workflow --namespace agentverse --name ci-review-pipeline \
  --format argo-workflow > ci-review.argo.yaml
```

## Standards Compatibility

| Standard           | Compatible? | Export format                           |
|--------------------|-------------|----------------------------------------|
| GitHub Actions     | ✅           | `--format github-actions`              |
| Argo Workflows     | ✅           | `--format argo-workflow`               |
| Prefect            | ✅           | `--format prefect`                     |
| Apache Airflow DAG | ✅           | `--format airflow`                     |
| LangGraph          | ✅           | `--format langgraph`                   |

## Notes

- Steps without `depends_on` run in **parallel** by default.
- `{{trigger.*}}` variables are injected by the workflow runtime.
- Add `retry` and `timeout` per step for production resilience.
- Combine with `soul` artifacts to give each agent step a consistent persona.

