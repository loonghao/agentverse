//! Parsing of `SKILL.md` YAML frontmatter.
//!
//! Supports all AgentVerse artifact kinds (skill, soul, prompt, workflow, agent)
//! with both simple and extended metadata:
//!
//! ```markdown
//! ---
//! name: my-skill
//! kind: skill
//! description: Does a thing.
//! version: "0.1.0"
//! tags: [ci, testing]
//! license: MIT
//! homepage: https://example.com
//! author: someone
//! metadata:
//!   openclaw:
//!     requires:
//!       env:
//!         - MY_API_KEY
//!       bins:
//!         - curl
//! ---
//! ```

use serde::Deserialize;

/// Structured result from parsing a `SKILL.md` frontmatter block.
#[derive(Debug, Clone)]
pub struct ParsedSkillMd {
    /// Artifact slug, e.g. `my-skill`.
    pub name: String,
    /// Registry namespace (user or org), e.g. `myorg`.
    /// Required for `agentverse publish SKILL.md`.
    pub namespace: Option<String>,
    /// Artifact kind: `skill` | `soul` | `prompt` | `workflow` | `agent`.
    /// Defaults to `"skill"` when the `kind:` field is absent.
    pub kind: String,
    /// Short human-readable description.
    pub description: Option<String>,
    /// Suggested initial version string (e.g. `"0.1.0"`). The registry
    /// is the source of truth for versions; this is advisory only.
    pub version: Option<String>,
    /// Discovery tags, e.g. `["ci", "testing", "http"]`.
    pub tags: Vec<String>,
    /// Homepage or docs URL.
    pub homepage: Option<String>,
    /// SPDX license identifier, e.g. `"MIT"`.
    pub license: Option<String>,
    /// Artifact author handle or name.
    pub author: Option<String>,
    /// Capabilities block as JSON (`input_modalities`, `output_modalities`,
    /// `protocols`, `permissions`, etc.). Empty object when absent.
    pub capabilities: serde_json::Value,
    /// Dependencies block as JSON (skill slugs → version constraints).
    /// Empty object when absent.
    pub dependencies: serde_json::Value,
    /// Raw `metadata:` block preserved as JSON (includes `openclaw.*`,
    /// `clawdbot.*`, runtime requirements, install instructions, etc.)
    pub metadata: serde_json::Value,
}

impl Default for ParsedSkillMd {
    fn default() -> Self {
        Self {
            name: String::new(),
            namespace: None,
            kind: "skill".to_owned(),
            description: None,
            version: None,
            tags: Vec::new(),
            homepage: None,
            license: None,
            author: None,
            capabilities: serde_json::Value::Object(Default::default()),
            dependencies: serde_json::Value::Object(Default::default()),
            metadata: serde_json::Value::Null,
        }
    }
}

/// Internal serde target for the raw frontmatter YAML.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct RawFrontmatter {
    name: Option<String>,
    /// Registry namespace (user or org). Required for `publish`.
    namespace: Option<String>,
    /// Artifact kind: skill | soul | prompt | workflow | agent.
    kind: Option<String>,
    description: Option<String>,
    /// Accept both quoted ("0.1.0") and unquoted (0.1.0) versions.
    version: Option<serde_yaml::Value>,
    tags: Vec<String>,
    homepage: Option<String>,
    license: Option<String>,
    author: Option<String>,
    /// Structured capability declarations (input/output modalities, protocols…)
    capabilities: Option<serde_yaml::Value>,
    /// Skill-level dependencies: skill slug → version constraint.
    dependencies: Option<serde_yaml::Value>,
    metadata: Option<serde_yaml::Value>,
}

/// Parse a `SKILL.md` string and extract structured frontmatter.
///
/// `fallback_name` is used when the frontmatter is absent or the `name` field
/// is missing; typically the last path segment of the skill directory.
pub fn parse_skill_md(content: &str, fallback_name: &str) -> ParsedSkillMd {
    let fm_text = extract_frontmatter(content);
    if fm_text.is_empty() {
        return ParsedSkillMd {
            name: fallback_name.to_owned(),
            ..Default::default()
        };
    }

    let raw: RawFrontmatter = serde_yaml::from_str(&fm_text).unwrap_or_default();

    let metadata = match raw.metadata {
        Some(v) => yaml_to_json(v),
        None => serde_json::Value::Null,
    };

    // Extract homepage from metadata.openclaw.homepage if not set at top level
    let homepage = raw.homepage.or_else(|| {
        metadata
            .get("openclaw")
            .and_then(|o| o.get("homepage"))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    });

    // Validate and normalise the kind field; default to "skill".
    let kind = raw
        .kind
        .map(|k| k.to_lowercase())
        .filter(|k| {
            matches!(
                k.as_str(),
                "skill" | "soul" | "prompt" | "workflow" | "agent"
            )
        })
        .unwrap_or_else(|| "skill".to_owned());

    let empty_obj = || serde_json::Value::Object(Default::default());

    ParsedSkillMd {
        name: raw
            .name
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| fallback_name.to_owned()),
        namespace: raw.namespace,
        kind,
        description: raw.description,
        version: raw.version.and_then(yaml_scalar_to_string),
        tags: raw.tags,
        homepage,
        license: raw.license,
        author: raw.author,
        capabilities: raw.capabilities.map(yaml_to_json).unwrap_or_else(empty_obj),
        dependencies: raw.dependencies.map(yaml_to_json).unwrap_or_else(empty_obj),
        metadata,
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Extract the raw text between the opening `---` and the closing `---`.
fn extract_frontmatter(content: &str) -> String {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return String::new();
    }
    // Skip the opening `---` line
    let after_open = match trimmed.find('\n') {
        Some(i) => &trimmed[i + 1..],
        None => return String::new(),
    };
    // Find the closing `---`
    match after_open.find("\n---") {
        Some(end) => after_open[..end].to_owned(),
        None => String::new(),
    }
}

/// Recursively convert a `serde_yaml::Value` to `serde_json::Value`.
fn yaml_to_json(v: serde_yaml::Value) -> serde_json::Value {
    match v {
        serde_yaml::Value::Null => serde_json::Value::Null,
        serde_yaml::Value::Bool(b) => serde_json::Value::Bool(b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::Number(i.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        serde_yaml::Value::String(s) => serde_json::Value::String(s),
        serde_yaml::Value::Sequence(seq) => {
            serde_json::Value::Array(seq.into_iter().map(yaml_to_json).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, val) in map {
                let key = match k {
                    serde_yaml::Value::String(s) => s,
                    other => format!("{other:?}"),
                };
                obj.insert(key, yaml_to_json(val));
            }
            serde_json::Value::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_json(tagged.value),
    }
}

/// Convert a YAML scalar value to its string representation.
fn yaml_scalar_to_string(v: serde_yaml::Value) -> Option<String> {
    match v {
        serde_yaml::Value::String(s) => Some(s),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE: &str = "---\nname: api-smoke-tester\ndescription: Runs smoke tests.\ntags: [testing, ci, api]\nversion: \"0.1.0\"\nauthor: agentverse-ci\nlicense: MIT\n---\n# Body";

    const OPENCLAW: &str = "---\nname: agentverse-cli\ndescription: \"Manage AI skills.\"\nversion: 0.1.4\nmetadata:\n  openclaw:\n    homepage: https://github.com/loonghao/agentverse\n    requires:\n      bins:\n        - agentverse\n    install:\n      - kind: shell\n        linux: \"curl -fsSL https://example.com/install.sh | bash\"\n---\n# CLI";

    const NO_FRONTMATTER: &str = "# Just markdown\nNo frontmatter.";

    const MULTILINE_TAGS: &str = "---\nname: multiline-skill\ndescription: Multiline tags.\ntags:\n  - search\n  - code\n  - analysis\n---";

    const MISSING_NAME: &str = "---\ndescription: No name field.\ntags: [orphan]\n---";

    const SOUL_SKILL: &str = "---\nname: empathetic-counselor\nkind: soul\ndescription: A warm soul persona.\nversion: \"0.1.0\"\ntags: [soul, openclaw]\n---\n# Soul";

    const WORKFLOW_SKILL: &str = "---\nname: ci-review-pipeline\nkind: workflow\ndescription: Agent-driven CI review.\nversion: \"0.1.0\"\ntags: [workflow, ci]\n---\n# Workflow";

    const UNKNOWN_KIND: &str =
        "---\nname: weird-thing\nkind: banana\ndescription: Unknown kind.\n---";

    #[test]
    fn parse_simple_fields() {
        let p = parse_skill_md(SIMPLE, "fallback");
        assert_eq!(p.name, "api-smoke-tester");
        assert_eq!(p.kind, "skill");
        assert_eq!(p.description.as_deref(), Some("Runs smoke tests."));
        assert_eq!(p.version.as_deref(), Some("0.1.0"));
        assert_eq!(p.tags, ["testing", "ci", "api"]);
        assert_eq!(p.author.as_deref(), Some("agentverse-ci"));
        assert_eq!(p.license.as_deref(), Some("MIT"));
        assert!(p.metadata.is_null());
    }

    #[test]
    fn parse_openclaw_metadata_block() {
        let p = parse_skill_md(OPENCLAW, "fallback");
        assert_eq!(p.name, "agentverse-cli");
        assert_eq!(p.version.as_deref(), Some("0.1.4"));
        assert!(p.tags.is_empty());
        // homepage should be promoted from metadata.openclaw.homepage
        assert_eq!(
            p.homepage.as_deref(),
            Some("https://github.com/loonghao/agentverse")
        );
        let meta = &p.metadata;
        assert!(meta.is_object(), "metadata must be a JSON object");
        assert_eq!(meta["openclaw"]["requires"]["bins"][0], "agentverse");
    }

    #[test]
    fn fallback_when_no_frontmatter() {
        let p = parse_skill_md(NO_FRONTMATTER, "my-fallback");
        assert_eq!(p.name, "my-fallback");
        assert!(p.description.is_none());
        assert!(p.tags.is_empty());
    }

    #[test]
    fn multiline_tags() {
        let p = parse_skill_md(MULTILINE_TAGS, "fallback");
        assert_eq!(p.name, "multiline-skill");
        assert_eq!(p.tags, ["search", "code", "analysis"]);
    }

    #[test]
    fn fallback_name_when_name_missing() {
        let p = parse_skill_md(MISSING_NAME, "path-fallback");
        assert_eq!(p.name, "path-fallback");
        assert_eq!(p.kind, "skill");
        assert_eq!(p.description.as_deref(), Some("No name field."));
        assert_eq!(p.tags, ["orphan"]);
    }

    #[test]
    fn kind_soul_is_extracted() {
        let p = parse_skill_md(SOUL_SKILL, "fallback");
        assert_eq!(p.name, "empathetic-counselor");
        assert_eq!(p.kind, "soul");
        assert_eq!(p.tags, ["soul", "openclaw"]);
    }

    #[test]
    fn kind_workflow_is_extracted() {
        let p = parse_skill_md(WORKFLOW_SKILL, "fallback");
        assert_eq!(p.name, "ci-review-pipeline");
        assert_eq!(p.kind, "workflow");
    }

    #[test]
    fn unknown_kind_defaults_to_skill() {
        let p = parse_skill_md(UNKNOWN_KIND, "fallback");
        assert_eq!(p.name, "weird-thing");
        assert_eq!(p.kind, "skill", "unrecognised kind must default to 'skill'");
    }

    #[test]
    fn no_frontmatter_kind_defaults_to_skill() {
        let p = parse_skill_md(NO_FRONTMATTER, "my-fallback");
        assert_eq!(p.kind, "skill");
    }
}
