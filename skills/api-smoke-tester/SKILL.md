---
name: api-smoke-tester
description: Runs a suite of smoke tests against any REST API and reports status, latency, and failure details.
tags: [testing, ci, api, http]
version: "0.1.0"
author: agentverse-ci
license: MIT
---

# API Smoke Tester

A CI-oriented skill that validates REST API availability, correctness, and performance.
Ideal for post-deploy sanity checks and continuous health monitoring.

## When to use this skill

- After every deployment to verify all critical endpoints are reachable
- Inside GitHub Actions / GitLab CI to gate merges on API health
- To detect regressions in response schemas or HTTP status codes

## Inputs

Provide a list of endpoint specifications:

```json
{
  "base_url": "https://api.example.com",
  "endpoints": [
    { "method": "GET",  "path": "/health",          "expect_status": 200 },
    { "method": "POST", "path": "/api/v1/auth/login","expect_status": 200,
      "body": { "username": "ci-bot", "password": "{{ CI_PASSWORD }}" } },
    { "method": "GET",  "path": "/api/v1/skills",   "expect_status": 200 }
  ],
  "timeout_ms": 5000,
  "fail_fast": false
}
```

## Steps

1. For each endpoint, send the HTTP request with the specified method and body.
2. Compare the actual HTTP status code against `expect_status`.
3. Optionally validate that the response body matches a JSON schema (`expect_schema`).
4. Measure response latency and flag endpoints exceeding `timeout_ms`.
5. Aggregate results into a report:
   - ✅ **PASS** – status matches, latency within budget
   - ⚠️ **SLOW** – status matches but latency exceeded threshold
   - ❌ **FAIL** – wrong status code or connection error

## Output format

```json
{
  "summary": { "total": 3, "passed": 2, "failed": 1, "duration_ms": 142 },
  "results": [
    { "path": "/health",      "status": 200, "latency_ms": 12,  "result": "pass" },
    { "path": "/auth/login",  "status": 200, "latency_ms": 87,  "result": "pass" },
    { "path": "/api/v1/skills","status": 503, "latency_ms": 32, "result": "fail",
      "error": "Service Unavailable" }
  ]
}
```

## CI integration example (GitHub Actions)

```yaml
- name: API Smoke Test
  uses: agentverse/run-skill@v1
  with:
    skill: agentverse-ci/api-smoke-tester
    inputs: |
      base_url: ${{ env.API_URL }}
      endpoints:
        - { method: GET, path: /health, expect_status: 200 }
```

## Notes

- Secrets in `body` values can be templated with `{{ ENV_VAR }}` syntax.
- Set `fail_fast: true` to abort after the first failure.
- For authenticated flows, chain with the `auth-token-injector` skill.

