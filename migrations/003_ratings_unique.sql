-- Migration 003: Enforce one rating per user per artifact (upsert safety)
-- This enables ON CONFLICT (artifact_id, user_id) DO UPDATE in add_rating.

CREATE UNIQUE INDEX IF NOT EXISTS uq_ratings_artifact_user
    ON ratings (artifact_id, user_id);

