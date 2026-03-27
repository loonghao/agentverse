use serde::{Deserialize, Serialize};

/// Reason for a version bump; determines which semver component increments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionBump {
    /// Backward-incompatible change (schema break, param removal/rename)
    Major,
    /// New backward-compatible capability added
    Minor,
    /// Backward-compatible fix, doc update, or performance improvement
    Patch,
}

/// The semantic version engine.
pub struct VersionEngine;

impl VersionEngine {
    /// Compute the next version string given the current version and bump type.
    ///
    /// # Examples
    /// ```
    /// use agentverse_core::versioning::{VersionBump, VersionEngine};
    /// assert_eq!(VersionEngine::bump("1.2.3", VersionBump::Patch).unwrap(), "1.2.4");
    /// assert_eq!(VersionEngine::bump("1.2.3", VersionBump::Minor).unwrap(), "1.3.0");
    /// assert_eq!(VersionEngine::bump("1.2.3", VersionBump::Major).unwrap(), "2.0.0");
    /// ```
    pub fn bump(current: &str, bump: VersionBump) -> Result<String, semver::Error> {
        let mut v = semver::Version::parse(current)?;
        match bump {
            VersionBump::Patch => {
                v.patch += 1;
            }
            VersionBump::Minor => {
                v.minor += 1;
                v.patch = 0;
            }
            VersionBump::Major => {
                v.major += 1;
                v.minor = 0;
                v.patch = 0;
            }
        }
        // Clear pre-release and build metadata on bump
        v.pre = semver::Prerelease::EMPTY;
        v.build = semver::BuildMetadata::EMPTY;
        Ok(v.to_string())
    }

    /// Infer the appropriate bump type by comparing old and new manifest JSON.
    /// Rules (in priority order):
    ///   1. If `capabilities.protocols` or `capabilities.permissions` shrank → Major
    ///   2. If `capabilities` gained new entries → Minor
    ///   3. Otherwise → Patch
    pub fn infer_bump(
        old_manifest: &serde_json::Value,
        new_manifest: &serde_json::Value,
    ) -> VersionBump {
        let old_caps = &old_manifest["capabilities"];
        let new_caps = &new_manifest["capabilities"];

        // Check for breaking: protocols or permissions removed
        if Self::array_shrank(old_caps, new_caps, "protocols")
            || Self::array_shrank(old_caps, new_caps, "permissions")
        {
            return VersionBump::Major;
        }

        // Check for additive: new protocols or permissions
        if Self::array_grew(old_caps, new_caps, "protocols")
            || Self::array_grew(old_caps, new_caps, "permissions")
            || Self::array_grew(old_caps, new_caps, "input_modalities")
            || Self::array_grew(old_caps, new_caps, "output_modalities")
        {
            return VersionBump::Minor;
        }

        VersionBump::Patch
    }

    fn array_shrank(old: &serde_json::Value, new: &serde_json::Value, key: &str) -> bool {
        let old_arr = Self::str_set(old, key);
        let new_arr = Self::str_set(new, key);
        old_arr.iter().any(|v| !new_arr.contains(v))
    }

    fn array_grew(old: &serde_json::Value, new: &serde_json::Value, key: &str) -> bool {
        let old_arr = Self::str_set(old, key);
        let new_arr = Self::str_set(new, key);
        new_arr.iter().any(|v| !old_arr.contains(v))
    }

    fn str_set(obj: &serde_json::Value, key: &str) -> std::collections::HashSet<String> {
        obj[key]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bump_patch() {
        assert_eq!(
            VersionEngine::bump("1.2.3", VersionBump::Patch).unwrap(),
            "1.2.4"
        );
    }

    #[test]
    fn test_bump_minor() {
        assert_eq!(
            VersionEngine::bump("1.2.3", VersionBump::Minor).unwrap(),
            "1.3.0"
        );
    }

    #[test]
    fn test_bump_major() {
        assert_eq!(
            VersionEngine::bump("1.2.3", VersionBump::Major).unwrap(),
            "2.0.0"
        );
    }

    #[test]
    fn test_infer_bump_breaking() {
        let old = serde_json::json!({"capabilities": {"protocols": ["mcp", "rest"]}});
        let new = serde_json::json!({"capabilities": {"protocols": ["mcp"]}});
        assert_eq!(VersionEngine::infer_bump(&old, &new), VersionBump::Major);
    }

    #[test]
    fn test_infer_bump_additive() {
        let old = serde_json::json!({"capabilities": {"protocols": ["mcp"]}});
        let new = serde_json::json!({"capabilities": {"protocols": ["mcp", "rest"]}});
        assert_eq!(VersionEngine::infer_bump(&old, &new), VersionBump::Minor);
    }
}
