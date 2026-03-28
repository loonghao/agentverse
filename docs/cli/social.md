# Social Commands

AgentVerse includes a full social layer: likes, ratings, comments, and stats.

All social commands accept the artifact as `<kind>/<namespace>/<name>`.

## like / unlike

```bash
# Like an artifact
agentverse like skill/myorg/my-skill
# ♥ Liked skill/myorg/my-skill

# Remove your like
agentverse unlike skill/myorg/my-skill
# ✓ Unliked skill/myorg/my-skill
```

## rate

Rate an artifact 1–5 stars, with an optional review:

```bash
agentverse rate <kind>/<namespace>/<name> <SCORE> [--review <TEXT>]
```

| Argument | Description |
|----------|-------------|
| `<SCORE>` | Integer 1–5 |
| `--review <TEXT>` | Optional review text |

### Examples

```bash
agentverse rate skill/myorg/my-skill 5
agentverse rate workflow/ops/deploy-pipeline 4 --review "Works great, minor docs issue"
```

Each user can rate an artifact once. A second `rate` command on the same artifact will update the existing rating.

## comment

Post a comment on an artifact:

```bash
agentverse comment <kind>/<namespace>/<name> "<TEXT>"
```

### Examples

```bash
agentverse comment skill/myorg/my-skill "Works perfectly in production!"
agentverse comment agent/myorg/support-bot "Integration with Slack works great"
```

Threaded replies and comment management (edit, delete) are available via the REST API.

## stats

Show social statistics for an artifact:

```bash
agentverse stats <kind>/<namespace>/<name>
```

**Output includes:**
- Total likes
- Average star rating and rating count
- Total comments
- Download count

### Example

```bash
agentverse stats skill/myorg/my-skill
# skill/myorg/my-skill
#   ♥ Likes:    42
#   ★ Rating:   4.7 / 5  (23 ratings)
#   💬 Comments: 8
#   ↓ Downloads: 1,234
```

