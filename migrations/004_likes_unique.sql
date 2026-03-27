-- Migration 004: Enforce one like per user per artifact (idempotent upsert safety)
-- Enables ON CONFLICT (artifact_id, user_id) DO NOTHING in add_like.

CREATE UNIQUE INDEX IF NOT EXISTS uq_likes_artifact_user
    ON likes (artifact_id, user_id);

