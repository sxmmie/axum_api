-- Generic trigger function reused by every table that has updated_at.
-- Defining it once here keeps every subsequent migration a one-liner.
CREATE OR REPLACE FUNCTION set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Case-insensitive uniqueness: "user@x.com" and "USER@x.com" are the same
-- account. Enforced at the DB level so it holds even under race conditions
-- two concurrent registration requests can't both slip through app-level
-- checks.
CREATE UNIQUE INDEX users_email_unique_idx ON users (lower(email));

CREATE TRIGGER users_set_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW
    EXECUTE FUNCTION set_updated_at();
