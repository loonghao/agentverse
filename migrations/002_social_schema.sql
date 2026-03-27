-- ============================================================
-- Social: comments (reviews, learning reports, benchmarks)
-- ============================================================
CREATE TABLE comments (
    id               UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    artifact_id      UUID NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
    version_id       UUID REFERENCES artifact_versions(id) ON DELETE SET NULL,
    author_id        UUID NOT NULL REFERENCES users(id),
    parent_id        UUID REFERENCES comments(id) ON DELETE CASCADE,
    content          TEXT NOT NULL,
    kind             VARCHAR(16) NOT NULL DEFAULT 'review'
                         CHECK (kind IN ('review', 'learning', 'suggestion', 'bug', 'benchmark')),
    likes_count      BIGINT NOT NULL DEFAULT 0,
    -- Structured benchmark data for agent-submitted performance reports
    benchmark_payload JSONB,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_comments_artifact ON comments(artifact_id);
CREATE INDEX idx_comments_version  ON comments(version_id);
CREATE INDEX idx_comments_author   ON comments(author_id);
CREATE INDEX idx_comments_kind     ON comments(kind);
CREATE INDEX idx_comments_parent   ON comments(parent_id);

-- ============================================================
-- Likes / upvotes on artifacts and versions
-- ============================================================
CREATE TABLE likes (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    artifact_id UUID NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
    version_id  UUID REFERENCES artifact_versions(id) ON DELETE SET NULL,
    user_id     UUID NOT NULL REFERENCES users(id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (artifact_id, user_id)
);

CREATE INDEX idx_likes_artifact ON likes(artifact_id);
CREATE INDEX idx_likes_user     ON likes(user_id);

-- ============================================================
-- Ratings (1-5 stars)
-- ============================================================
CREATE TABLE ratings (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    artifact_id UUID NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
    version_id  UUID REFERENCES artifact_versions(id) ON DELETE SET NULL,
    user_id     UUID NOT NULL REFERENCES users(id),
    score       SMALLINT NOT NULL CHECK (score BETWEEN 1 AND 5),
    review_text TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (artifact_id, user_id)
);

CREATE INDEX idx_ratings_artifact ON ratings(artifact_id);

-- Materialized view for fast average rating queries
CREATE MATERIALIZED VIEW artifact_rating_summary AS
    SELECT
        artifact_id,
        COUNT(*)            AS total_ratings,
        AVG(score)::NUMERIC(3,2) AS avg_score
    FROM ratings
    GROUP BY artifact_id;

CREATE UNIQUE INDEX idx_rating_summary_artifact ON artifact_rating_summary(artifact_id);

-- ============================================================
-- Agent interactions: learn / fork / cite / benchmark
-- ============================================================
CREATE TABLE agent_interactions (
    id               UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    from_agent_id    UUID NOT NULL REFERENCES users(id),
    artifact_id      UUID NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
    version_id       UUID REFERENCES artifact_versions(id) ON DELETE SET NULL,
    kind             VARCHAR(16) NOT NULL
                         CHECK (kind IN ('learn', 'fork', 'cite', 'benchmark')),
    payload          JSONB NOT NULL DEFAULT '{}',
    confidence_score DOUBLE PRECISION CHECK (confidence_score BETWEEN 0.0 AND 1.0),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_interactions_agent    ON agent_interactions(from_agent_id);
CREATE INDEX idx_interactions_artifact ON agent_interactions(artifact_id);
CREATE INDEX idx_interactions_kind     ON agent_interactions(kind);

-- ============================================================
-- Event store: CQRS append-only audit log
-- ============================================================
CREATE TABLE events (
    id             UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    aggregate_type VARCHAR(64) NOT NULL,
    aggregate_id   UUID NOT NULL,
    event_type     VARCHAR(128) NOT NULL,
    payload        JSONB NOT NULL,
    sequence       BIGINT NOT NULL,
    occurred_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    UNIQUE (aggregate_id, sequence)
);

CREATE INDEX idx_events_aggregate ON events(aggregate_type, aggregate_id);
CREATE INDEX idx_events_type      ON events(event_type);
CREATE INDEX idx_events_occurred  ON events(occurred_at DESC);

