-- ============================================================
-- Add 'github_repo' value to the source_type ENUM.
--
-- The original migration (005) only defined clawhub / github / url.
-- The GitHubRepo backend introduced in the skills management feature
-- stores subdirectory-based GitHub skills and needs its own source type.
-- ============================================================

ALTER TYPE source_type ADD VALUE IF NOT EXISTS 'github_repo';

-- ============================================================
-- Composite index for common package lookup: artifact version +
-- source type. Useful when filtering packages by multiple backends.
-- ============================================================

CREATE INDEX IF NOT EXISTS idx_skill_packages_version_source
    ON skill_packages(artifact_version_id, source_type);

-- ============================================================
-- Skill install history: additional index keyed by install time
-- to support "recently installed" queries.
-- ============================================================

CREATE INDEX IF NOT EXISTS idx_skill_installs_installed_at
    ON skill_installs(installed_at DESC);

