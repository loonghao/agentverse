# GitHub Releases Storage

The `github` backend uploads package archives as assets on a dedicated GitHub repository's releases. This is a great zero-infrastructure option for open-source projects.

## How It Works

1. Server creates a new GitHub Release named after the skill version
2. The zip archive is uploaded as a release asset
3. The publicly accessible asset URL is stored and returned to the CLI
4. Users can download packages without any credentials (public repos)

## Configuration

```toml
[object_store]
backend = "github"

[object_store.github]
# GitHub organization or username that owns the storage repo
owner = "myorg"
# Repository where releases (and package assets) will be created
repo  = "agentverse-packages"
# Personal Access Token or Fine-Grained Token with `contents: write`
# Falls back to the GITHUB_TOKEN environment variable when omitted
token = "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
```

## Required Token Permissions

Create a **Fine-Grained Personal Access Token** (recommended) or a Classic PAT:

**Fine-Grained PAT:**
- Repository: `agentverse-packages`
- Permissions: **Contents → Read and Write**

**Classic PAT:**
- Scope: `repo` (full repository access)

### Setting the Token via Environment Variable

```bash
# In production, prefer environment variable over config file
export GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

## Setup: Create the Storage Repository

1. Create a **public** (or private) GitHub repository for storing packages:
   ```
   https://github.com/myorg/agentverse-packages
   ```
2. Initialize with a README (required to create the default branch)
3. The server will create releases automatically

## Example: Open-Source Workflow

For fully open-source projects, you can use your **main repo** (e.g., GitHub Actions token):

```toml
[object_store]
backend = "github"

[object_store.github]
owner = "myorg"
repo  = "agentverse"
# token falls back to GITHUB_TOKEN env var (set automatically in GitHub Actions)
```

In GitHub Actions:

```yaml
jobs:
  publish:
    runs-on: ubuntu-latest
    permissions:
      contents: write   # required for creating releases and uploading assets
    steps:
      - uses: actions/checkout@v4
      - name: Publish skill
        env:
          AGENTVERSE_TOKEN: ${{ secrets.AGENTVERSE_TOKEN }}
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}  # auto-set by GitHub Actions
        run: agentverse publish --zip skill.zip
```

## Release Naming

Releases are created with the tag:
```
<namespace>/<name>@<version>
```

Example: `myorg/code-linter@1.2.0`

## Limitations

- **API rate limits** — GitHub API has rate limits (5,000 req/hr for authenticated)
- **Asset size** — GitHub has a 2 GB per-asset limit
- **Private repos** — downloads require authentication if the repo is private
- **Deletion** — deleting a release asset is not reversible via the standard API

