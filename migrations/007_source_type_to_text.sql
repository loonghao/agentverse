-- ============================================================
-- Convert skill_packages.source_type from a PostgreSQL custom
-- ENUM type to plain TEXT.
--
-- Rationale
-- ---------
-- SeaORM/sqlx binds Rust `String` values as PostgreSQL `text`
-- (OID 25).  When the target column is a custom ENUM type,
-- PostgreSQL rejects the implicit cast in parameterized queries,
-- causing the INSERT to fail with a type-mismatch error.
-- Because the publishing hook swallowed errors (non-fatal), this
-- resulted in packages appearing to register (HTTP 201) but
-- never being written to the database.
--
-- Using TEXT instead of a custom ENUM:
--   • removes the need for a new ALTER TYPE migration every time
--     a new source backend is added (e.g. 'internal')
--   • is consistent with how the Rust code already treats the
--     field (as a plain string)
--   • leaves all existing data intact (ENUM values are stored
--     as their text representation, so the cast is lossless)
-- ============================================================

-- Step 1: change the column type (lossless: enum → text is always valid)
ALTER TABLE skill_packages
    ALTER COLUMN source_type TYPE TEXT
    USING source_type::TEXT;

-- Step 2: drop the now-unused ENUM type
DROP TYPE IF EXISTS source_type;

