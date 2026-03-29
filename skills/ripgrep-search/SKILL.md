---
name: ripgrep-search
kind: skill
description: "Blazing-fast code and file search using ripgrep (rg). Recursively searches patterns across large codebases with .gitignore awareness, type filters, and context lines."
version: "0.1.0"
tags: [search, code, cli, productivity]
license: MIT
metadata:
  openclaw:
    homepage: https://github.com/BurntSushi/ripgrep
    emoji: "🔍"
    requires:
      bins:
        - rg
    install:
      - kind: shell
        linux: "cargo install ripgrep || apt-get install -y ripgrep"
        macos: "brew install ripgrep"
        windows: "winget install BurntSushi.ripgrep.MSVC"
---

# Ripgrep Search

Search code and files at blazing speed using [ripgrep](https://github.com/BurntSushi/ripgrep).

## When to use

- Finding all usages of a function or variable across a large codebase
- Locating TODO/FIXME comments across a project
- Searching within specific file types (Rust, Python, TypeScript, etc.)
- Replacing `grep -r` in CI scripts for 10-100× speed improvement

## Inputs

```json
{
  "pattern": "fn parse_",
  "path": "./src",
  "file_type": "rust",
  "context_lines": 3,
  "case_sensitive": false,
  "max_results": 50
}
```

| Field           | Required | Description                                        |
|-----------------|----------|----------------------------------------------------|
| `pattern`       | ✓        | Regex or literal pattern to search                 |
| `path`          | ✗        | Directory or file to search (default: current dir) |
| `file_type`     | ✗        | Filter by file type: `rust`, `py`, `ts`, etc.      |
| `context_lines` | ✗        | Lines of context before/after each match           |
| `case_sensitive`| ✗        | Default `false`                                    |
| `max_results`   | ✗        | Limit output lines (default: unlimited)            |

## Example commands

```bash
# Search for a pattern in Rust files
rg "fn parse_" --type rust -C 3

# Find all TODO comments
rg "TODO|FIXME|HACK" --glob "*.{rs,py,ts}"

# Search case-insensitively, limit output
rg -i "error handling" --max-count 20
```

## Output

```json
{
  "matches": [
    {
      "file": "src/skill_md.rs",
      "line": 64,
      "content": "pub fn parse_skill_md(content: &str, fallback_name: &str) -> ParsedSkillMd {"
    }
  ],
  "total_matches": 1,
  "elapsed_ms": 12
}
```

