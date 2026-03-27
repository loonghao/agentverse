-- Enable required PostgreSQL extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "vector";
CREATE EXTENSION IF NOT EXISTS "pg_trgm";  -- for fuzzy text search

-- ============================================================
-- Users table: supports both human developers and AI agents
-- ============================================================
CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    username        VARCHAR(128) NOT NULL UNIQUE,
    email           VARCHAR(255) UNIQUE,
    password_hash   VARCHAR(255),
    kind            VARCHAR(16) NOT NULL DEFAULT 'human'
                        CHECK (kind IN ('human', 'agent', 'system')),
    capabilities    JSONB,
    -- Ed25519 public key in hex for signed manifest verification
    public_key      VARCHAR(128),
    is_verified     BOOLEAN NOT NULL DEFAULT FALSE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_kind     ON users(kind);

-- ============================================================
-- Artifacts: the registry "packages" (skills, souls, agents, etc.)
-- ============================================================
CREATE TABLE artifacts (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    kind            VARCHAR(16) NOT NULL
                        CHECK (kind IN ('skill', 'soul', 'agent', 'workflow', 'prompt')),
    namespace       VARCHAR(128) NOT NULL,
    name            VARCHAR(128) NOT NULL,
    display_name    VARCHAR(256),
    description     TEXT NOT NULL DEFAULT '',
    manifest        JSONB NOT NULL DEFAULT '{}',
    status          VARCHAR(16) NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'deprecated', 'retired', 'revoked')),
    author_id       UUID NOT NULL REFERENCES users(id),
    downloads       BIGINT NOT NULL DEFAULT 0,
    -- Vector embedding for semantic search (1536 dims for OpenAI ada-002, or 384 for minilm)
    embedding       vector(384),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (kind, namespace, name)
);

CREATE INDEX idx_artifacts_kind           ON artifacts(kind);
CREATE INDEX idx_artifacts_namespace      ON artifacts(namespace);
CREATE INDEX idx_artifacts_status         ON artifacts(status);
CREATE INDEX idx_artifacts_author         ON artifacts(author_id);
CREATE INDEX idx_artifacts_downloads      ON artifacts(downloads DESC);
-- GIN index for JSONB manifest queries
CREATE INDEX idx_artifacts_manifest       ON artifacts USING GIN (manifest);
-- Full-text search on name + description
CREATE INDEX idx_artifacts_fts            ON artifacts USING GIN (
    to_tsvector('english', coalesce(name, '') || ' ' || coalesce(description, ''))
);
-- pgvector HNSW index for approximate nearest-neighbor search
CREATE INDEX idx_artifacts_embedding      ON artifacts USING hnsw (embedding vector_cosine_ops);

-- ============================================================
-- Tags: many-to-many artifact tags
-- ============================================================
CREATE TABLE artifact_tags (
    artifact_id UUID NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
    tag         VARCHAR(64) NOT NULL,
    PRIMARY KEY (artifact_id, tag)
);

CREATE INDEX idx_artifact_tags_tag ON artifact_tags(tag);

-- ============================================================
-- Artifact versions: immutable append-only version history
-- ============================================================
CREATE TABLE artifact_versions (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    artifact_id     UUID NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
    version         VARCHAR(64) NOT NULL,
    major           INTEGER NOT NULL,
    minor           INTEGER NOT NULL,
    patch           INTEGER NOT NULL,
    pre_release     VARCHAR(64),
    content         JSONB NOT NULL,
    checksum        VARCHAR(64) NOT NULL,          -- sha256 hex
    signature       VARCHAR(256),                  -- Ed25519 hex
    changelog       TEXT,
    bump_reason     VARCHAR(8) NOT NULL DEFAULT 'patch'
                        CHECK (bump_reason IN ('patch', 'minor', 'major')),
    published_by    UUID NOT NULL REFERENCES users(id),
    published_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (artifact_id, version)
);

CREATE INDEX idx_versions_artifact    ON artifact_versions(artifact_id);
CREATE INDEX idx_versions_semver      ON artifact_versions(artifact_id, major DESC, minor DESC, patch DESC);
CREATE INDEX idx_versions_published   ON artifact_versions(published_at DESC);

