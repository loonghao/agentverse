-- =============================================================================
-- Agent Memory & Usage System
-- =============================================================================
-- Design rationale:
--   Skills are *bound* to specific agent kinds (claude, augment, openclaw…).
--   Each binding tracks install metadata, usage frequency, and a memory state
--   that mirrors human memory: hot → warm → cold → archived → forgotten.
--   Usage events provide a fine-grained audit trail for future analytics.
-- =============================================================================

-- ── Agent-Skill Bindings ──────────────────────────────────────────────────────
-- One row per (agent_user_id, skill_artifact_id, agent_kind) triple.
-- An agent may have the same skill installed for multiple runtimes, e.g.
-- both "claude" and "augment" can install "skill/openai/code-review".
CREATE TABLE agent_skill_bindings (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    -- The agent (user.kind = 'agent') that owns this binding.
    agent_id        UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- The skill artifact being installed (must be kind = 'skill').
    artifact_id     UUID NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,

    -- Pinned artifact_version at install time.
    version_id      UUID NOT NULL REFERENCES artifact_versions(id),

    -- Which agent runtime type this binding targets.
    -- Mirrors AgentKind on the CLI (openclaw, codebuddy, workerbuddy, claude, augment, custom:…).
    agent_kind      VARCHAR(64) NOT NULL DEFAULT 'custom',

    -- Absolute filesystem path where the skill was extracted on the agent's host.
    install_path    TEXT NOT NULL DEFAULT '',

    -- Path to the archived backup (non-null when memory_state = 'archived').
    backup_path     TEXT,

    -- Lifecycle state — mirrors MemoryState in agentverse-core.
    memory_state    VARCHAR(16) NOT NULL DEFAULT 'hot'
                        CHECK (memory_state IN ('hot', 'warm', 'cold', 'archived', 'forgotten')),

    -- Usage counters maintained by the CLI on each `agentverse use` invocation.
    use_count       BIGINT NOT NULL DEFAULT 0,
    last_used_at    TIMESTAMPTZ,

    installed_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- One binding per (agent, skill, runtime) — the agent can reinstall with
    -- a newer version by updating version_id rather than inserting a duplicate.
    UNIQUE (agent_id, artifact_id, agent_kind)
);

CREATE INDEX idx_asb_agent        ON agent_skill_bindings(agent_id);
CREATE INDEX idx_asb_artifact     ON agent_skill_bindings(artifact_id);
CREATE INDEX idx_asb_memory_state ON agent_skill_bindings(memory_state);
CREATE INDEX idx_asb_last_used    ON agent_skill_bindings(last_used_at DESC NULLS LAST);

-- ── Skill Usage Events ────────────────────────────────────────────────────────
-- Append-only log of every time a skill is invoked / "touched" by an agent.
-- Enables time-series analysis: daily active skills, usage trends, etc.
CREATE TABLE skill_usage_events (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),

    binding_id  UUID NOT NULL REFERENCES agent_skill_bindings(id) ON DELETE CASCADE,

    -- Optional free-form context supplied by the CLI (e.g. the task description
    -- or calling agent session ID).  Stored as JSONB for flexibility.
    context     JSONB NOT NULL DEFAULT '{}',

    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_sue_binding    ON skill_usage_events(binding_id);
CREATE INDEX idx_sue_occurred   ON skill_usage_events(occurred_at DESC);

-- ── Trigger: keep updated_at current ─────────────────────────────────────────
CREATE OR REPLACE FUNCTION touch_updated_at()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_asb_updated_at
    BEFORE UPDATE ON agent_skill_bindings
    FOR EACH ROW EXECUTE FUNCTION touch_updated_at();

-- ── Helper view: active (non-forgotten) bindings with artifact metadata ───────
CREATE VIEW agent_active_skills AS
SELECT
    asb.id,
    asb.agent_id,
    u.username             AS agent_username,
    asb.agent_kind,
    a.kind                 AS artifact_kind,
    a.namespace,
    a.name                 AS skill_name,
    av.version,
    asb.install_path,
    asb.memory_state,
    asb.use_count,
    asb.last_used_at,
    asb.installed_at
FROM agent_skill_bindings asb
JOIN users             u  ON u.id  = asb.agent_id
JOIN artifacts         a  ON a.id  = asb.artifact_id
JOIN artifact_versions av ON av.id = asb.version_id
WHERE asb.memory_state != 'forgotten';
