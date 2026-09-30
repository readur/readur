-- Session state for issued tokens.
-- `token_version` is embedded in every issued JWT; bumping it invalidates all
-- outstanding tokens for the user (logout, password change, deactivation).
ALTER TABLE users ADD COLUMN IF NOT EXISTS token_version INTEGER NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS is_active BOOLEAN NOT NULL DEFAULT TRUE;
ALTER TABLE users ADD COLUMN IF NOT EXISTS password_changed_at TIMESTAMPTZ;

-- An external identity (issuer, subject) must map to exactly one account.
-- Skipped (with a notice) if an existing installation already contains
-- duplicates, which must then be resolved manually.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM users
        WHERE oidc_subject IS NOT NULL
        GROUP BY oidc_issuer, oidc_subject
        HAVING COUNT(*) > 1
    ) THEN
        RAISE NOTICE 'Duplicate (oidc_issuer, oidc_subject) rows found; unique index not created';
    ELSE
        CREATE UNIQUE INDEX IF NOT EXISTS idx_users_oidc_identity_unique
            ON users(oidc_issuer, oidc_subject)
            WHERE oidc_subject IS NOT NULL;
    END IF;
END $$;

-- Short-lived, single-use values used during login flows (OIDC state/PKCE
-- verifier/nonce, and one-time codes handing a session to the frontend).
-- Only a SHA-256 of the lookup key is stored.
CREATE TABLE IF NOT EXISTS auth_ephemeral (
    key_hash CHAR(64) PRIMARY KEY,
    kind VARCHAR(32) NOT NULL,
    payload JSONB NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_auth_ephemeral_expires_at ON auth_ephemeral(expires_at);
