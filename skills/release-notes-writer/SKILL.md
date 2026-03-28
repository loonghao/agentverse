---
name: release-notes-writer
description: Generates structured release notes from git commit history, PR titles, and linked issues. Supports Conventional Commits and custom formats.
tags: [release, ci, documentation, changelog]
version: "0.1.0"
author: agentverse-ci
license: MIT
---

# Release Notes Writer

Automatically generates well-formatted release notes from commit history,
pull request metadata, and issue trackers — ready for GitHub Releases,
CHANGELOG.md, or Slack notifications.

## When to use this skill

- On every `git tag` push to auto-generate and post GitHub Release notes
- Weekly digest of merged PRs to a team Slack channel
- Maintaining a `CHANGELOG.md` that follows Keep a Changelog conventions

## Inputs

```json
{
  "repo": "org/repo",
  "from_ref": "v1.2.0",
  "to_ref":   "v1.3.0",
  "format":   "markdown",
  "sections": ["breaking", "features", "fixes", "deps", "chores"],
  "include_authors": true,
  "issue_tracker": "github"
}
```

| Field             | Required | Description                                         |
|-------------------|----------|-----------------------------------------------------|
| `repo`            | ✓        | GitHub `owner/repo` slug                            |
| `from_ref`        | ✓        | Start tag, branch, or commit SHA                    |
| `to_ref`          | ✗        | End ref (defaults to `HEAD`)                        |
| `format`          | ✗        | `markdown` (default), `json`, `slack`               |
| `sections`        | ✗        | Which categories to emit                            |
| `include_authors` | ✗        | Attribute changes to PR authors                     |
| `issue_tracker`   | ✗        | `github` or `jira` for issue link resolution        |

## Conventional Commit mapping

| Prefix              | Section      |
|---------------------|--------------|
| `feat:` / `feat!:`  | Features / Breaking Changes |
| `fix:`              | Bug Fixes    |
| `deps:` / `chore:`  | Maintenance  |
| `docs:`             | Documentation|
| `perf:`             | Performance  |

## Output (markdown)

```markdown
## What's Changed in v1.3.0

### 🚀 Features
- Add skill import from GitHub repository URL (#142) @loonghao
- Support `github_repo` source type for directory-based skills (#138) @loonghao

### 🐛 Bug Fixes
- Fix `SourceType::GitHub` serialised as `git_hub` instead of `github` (#145)

### 📦 Dependencies
- Bump `zip` to 2.4.2
- Bump `axum` to 0.8.8

**Full Changelog**: https://github.com/org/repo/compare/v1.2.0...v1.3.0
```

## CI integration (GitHub Actions)

```yaml
on:
  push:
    tags: ['v*']

jobs:
  release:
    steps:
      - name: Generate Release Notes
        uses: agentverse/run-skill@v1
        with:
          skill: agentverse-ci/release-notes-writer
          inputs: |
            repo: ${{ github.repository }}
            from_ref: ${{ github.event.before }}
            to_ref:   ${{ github.ref_name }}
            format: markdown
        env:
          AGENTVERSE_TOKEN: ${{ secrets.AGENTVERSE_TOKEN }}
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          body: ${{ steps.generate.outputs.notes }}
```

## Notes

- Breaking changes (commits with `!` suffix or `BREAKING CHANGE:` footer) are always promoted to a dedicated section regardless of `sections` config.
- For Jira integration, provide `JIRA_BASE_URL` and `JIRA_TOKEN` as environment variables.

