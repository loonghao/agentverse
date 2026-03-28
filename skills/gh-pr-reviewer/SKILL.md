---
name: gh-pr-reviewer
description: "Fetch GitHub Pull Request diffs, metadata, and CI status using the gh CLI. Summarise changes, check review status, and post structured review comments."
version: "0.1.0"
tags: [github, pull-request, code-review, ci]
license: MIT
metadata:
  openclaw:
    homepage: https://cli.github.com
    emoji: "🐙"
    requires:
      bins:
        - gh
      env:
        - GH_TOKEN
    install:
      - kind: shell
        linux: "curl -fsSL https://cli.github.com/packages/githubcli-archive-keyring.gpg | sudo dd of=/usr/share/keyrings/githubcli-archive-keyring.gpg && sudo apt install gh"
        macos: "brew install gh"
        windows: "winget install GitHub.cli"
---

# GitHub PR Reviewer

Interact with GitHub Pull Requests via the [GitHub CLI (`gh`)](https://cli.github.com).

## When to use

- Reviewing a PR diff before merging
- Checking CI check status for a PR
- Posting structured review comments from an AI review pipeline
- Listing open PRs filtered by author, label, or reviewer

## Inputs

```json
{
  "repo": "loonghao/agentverse",
  "pr_number": 12,
  "action": "review",
  "comment": "LGTM — all checks pass."
}
```

| Field       | Required | Description                                              |
|-------------|----------|----------------------------------------------------------|
| `repo`      | ✓        | `owner/repo` slug                                        |
| `pr_number` | ✓        | Pull request number                                      |
| `action`    | ✓        | `diff` / `status` / `review` / `list`                   |
| `comment`   | ✗        | Review comment body (used when `action = "review"`)      |

## Example commands

```bash
# Fetch the diff for PR #12
gh pr diff 12 --repo loonghao/agentverse

# Check CI status
gh pr checks 12 --repo loonghao/agentverse

# Post a review comment
gh pr review 12 --approve --body "LGTM"

# List open PRs assigned to me
gh pr list --assignee @me --state open
```

## Output (action=status)

```json
{
  "pr": {
    "number": 12,
    "title": "feat: skills management system",
    "state": "open",
    "checks": [
      { "name": "quality", "state": "success" },
      { "name": "test (ubuntu)", "state": "success" },
      { "name": "e2e", "state": "success" }
    ],
    "mergeable": true
  }
}
```

