-- Personal API keys for programmatic authentication.
-- Keys are never stored in plaintext: `key_hash` is SHA-256 (hex) of the full
-- `readur_pat_<token>` string. `key_prefix` is stored separately so users can
-- identify a key in the UI without revealing the full value.
CREATE TABLE IF NOT EXISTS api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    key_hash CHAR(64) NOT NULL UNIQUE,
    key_prefix VARCHAR(16) NOT NULL,
    expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_key_hash_active ON api_keys(key_hash) WHERE revoked_at IS NULL;
