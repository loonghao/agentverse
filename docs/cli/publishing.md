# Publishing Commands

## publish

Publish a new artifact or a new version from a manifest file.

```bash
agentverse publish [MANIFEST] [OPTIONS]
```

| Argument / Flag | Default | Description |
|-----------------|---------|-------------|
| `[MANIFEST]` | `./agentverse.toml` | Path to the manifest file (TOML or JSON) |
| `--content <FILE>` | `<manifest-dir>/content.json` | Content file (JSON) |
| `--bump <LEVEL>` | _(auto-inferred)_ | `patch` \| `minor` \| `major` |
| `--changelog <MSG>` | — | Version changelog message |
| `--zip <FILE>` | — | Zip archive to upload to object store |

### First Publish

If the artifact does not yet exist, `publish` creates it at version `0.1.0`.

```bash
# Uses ./agentverse.toml and ./content.json by default
agentverse publish

# Explicit path
agentverse publish path/to/my-skill/agentverse.toml

# With changelog and explicit bump
agentverse publish --bump minor --changelog "Added streaming support"
```

### Republish (New Version)

If the artifact already exists, a conflict is detected and `publish` automatically posts a new version:

```bash
agentverse publish --bump patch --changelog "Fix edge case in parser"
agentverse publish --bump major --changelog "Breaking: renamed config key"
```

### Upload a Package Archive

Use `--zip` to upload a binary package (zip) to the server's configured object store (S3, COS, local, GitHub Releases, etc.):

```bash
# Build your skill
zip -r my-skill.zip src/ requirements.txt

# Publish metadata + upload zip in one step
agentverse publish --zip my-skill.zip --changelog "v1.2.0 package"
# ✓ Published skill/myorg/my-skill  v1.2.0
# ↑ Uploading my-skill.zip …
# ✓ Package uploaded  https://storage.example.com/myorg/my-skill/1.2.0.zip
```

The returned download URL is stored in the skill's package registry.

## update

Update an existing artifact's display name or manifest:

```bash
agentverse update <kind>/<namespace>/<name> [OPTIONS]
# (flags depend on server API — use --help for details)
```

## fork

Fork an artifact into a new namespace or name:

```bash
agentverse fork <kind>/<namespace>/<name> \
  --new-namespace <TARGET_NS> \
  --new-name <TARGET_NAME>
```

### Example

```bash
agentverse fork skill/python-tools/linter \
  --new-namespace myorg \
  --new-name my-custom-linter
# ✓ Forked skill/python-tools/linter → skill/myorg/my-custom-linter  v0.1.0
```

The fork starts at version `0.1.0` and is independently versioned.

## deprecate

Soft-delete (deprecate) an artifact — it remains accessible but is marked deprecated:

```bash
agentverse deprecate <kind>/<namespace>/<name> [--reason <MSG>]
```

### Example

```bash
agentverse deprecate skill/myorg/old-linter \
  --reason "Superseded by myorg/new-linter"
```

Users who fetch the artifact will see the deprecation notice.

