-- Secrets the server generates for itself, such as the JWT signing key used
-- when JWT_SECRET is not set. One row per named secret; replicas starting at
-- the same time converge on whichever value is inserted first.
CREATE TABLE IF NOT EXISTS server_secrets (
    name TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
