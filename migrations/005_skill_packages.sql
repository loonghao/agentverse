-- ============================================================
-- Skill packages: tracks downloadable package artifacts with
-- multi-backend source metadata (clawhub, github, url).
-- One skill version may have multiple packages (e.g. per-platform).
-- ============================================================

CREATE TYPE source_type AS ENUM ('clawhub', 'github', 'url');

CREATE TABLE skill_packages (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    -- Links to a specific artifact_version of kind='skill'
    artifact_version_id UUID NOT NULL REFERENCES artifact_versions(id) ON DELETE CASCADE,
    source_type         source_type NOT NULL,
    -- Canonical download URL resolved at publish time
    download_url        TEXT NOT NULL,
    -- Optional SHA-256 hex checksum of the package archive
    checksum            VARCHAR(64),
    -- Uncompressed file size in bytes (informational)
    file_size           BIGINT,
    -- Extra metadata: platform, arch, agent compatibility hints
    metadata            JSONB NOT NULL DEFAULT '{}',
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- Each version may have at most one package per source_type
    UNIQUE (artifact_version_id, source_type)
);

CREATE INDEX idx_skill_packages_version ON skill_packages(artifact_version_id);
CREATE INDEX idx_skill_packages_source  ON skill_packages(source_type);

-- ============================================================
-- Skill install records: tracks which agent has which skill
-- installed, and where on the filesystem it was deployed.
-- ============================================================

CREATE TABLE skill_installs (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    skill_package_id UUID NOT NULL REFERENCES skill_packages(id) ON DELETE CASCADE,
    -- The agent kind: openclaw, codebuddy, workerbuddy, claude, augment, ...
    agent_kind      VARCHAR(32) NOT NULL,
    -- Absolute filesystem path where the skill was extracted
    install_path    TEXT NOT NULL,
    installed_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (skill_package_id, agent_kind)
);

CREATE INDEX idx_skill_installs_pkg   ON skill_installs(skill_package_id);
CREATE INDEX idx_skill_installs_agent ON skill_installs(agent_kind);

