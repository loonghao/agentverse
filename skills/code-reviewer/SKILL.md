---
name: code-reviewer
description: Reviews a pull request or diff and produces structured feedback on correctness, security, performance, and style.
tags: [code-review, ci, security, quality]
version: "0.1.0"
author: agentverse-ci
license: MIT
---

# Code Reviewer

An AI-powered code review skill that analyses diffs and produces actionable,
structured feedback — categorised by severity and type.

## When to use this skill

- Automated first-pass review on every PR before human reviewers are assigned
- Security scanning for common vulnerability patterns (injection, secrets in code, SSRF)
- Style and convention enforcement without maintaining a linter config

## Inputs

```json
{
  "diff": "<unified-diff string or GitHub PR URL>",
  "language": "rust",
  "rules": ["security", "performance", "style", "correctness"],
  "severity_threshold": "warning"
}
```

| Field                | Required | Description                                          |
|----------------------|----------|------------------------------------------------------|
| `diff`               | ✓        | Unified diff text or GitHub PR URL                   |
| `language`           | ✗        | Hint for syntax-aware analysis (auto-detected)       |
| `rules`              | ✗        | Subset of review categories to run                   |
| `severity_threshold` | ✗        | Minimum severity to include: `info`/`warning`/`error`|

## Review categories

| Category        | Examples                                              |
|-----------------|-------------------------------------------------------|
| `security`      | SQL injection, hardcoded secrets, SSRF, path traversal|
| `correctness`   | Off-by-one, unchecked unwrap, wrong type conversions  |
| `performance`   | N+1 queries, unnecessary clones, blocking async calls |
| `style`         | Naming conventions, dead code, missing docs           |

## Output format

```json
{
  "summary": "3 issues found (1 error, 2 warnings)",
  "issues": [
    {
      "file": "src/handlers/auth.rs",
      "line": 42,
      "severity": "error",
      "category": "security",
      "message": "JWT secret read from environment without fallback validation",
      "suggestion": "Validate that JWT_SECRET is non-empty at startup and reject empty strings."
    },
    {
      "file": "src/db/query.rs",
      "line": 88,
      "severity": "warning",
      "category": "performance",
      "message": "N+1 query pattern detected inside loop",
      "suggestion": "Batch the query outside the loop using an IN clause."
    }
  ],
  "score": 74,
  "approved": false
}
```

## CI integration (GitHub Actions)

```yaml
- name: AI Code Review
  uses: agentverse/run-skill@v1
  with:
    skill: agentverse-ci/code-reviewer
    inputs: |
      diff: ${{ github.event.pull_request.url }}
      rules: [security, correctness]
      severity_threshold: warning
  env:
    AGENTVERSE_TOKEN: ${{ secrets.AGENTVERSE_TOKEN }}
```

## Notes

- The skill does **not** auto-approve PRs; it only produces feedback.
- Set `severity_threshold: error` to surface only blocking issues.
- Combine with `api-smoke-tester` for full pre-merge validation.

