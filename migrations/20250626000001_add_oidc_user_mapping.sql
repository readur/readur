-- Add OIDC support to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS oidc_subject VARCHAR(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS oidc_issuer VARCHAR(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS oidc_email VARCHAR(255);
ALTER TABLE users ADD COLUMN IF NOT EXISTS auth_provider VARCHAR(50) DEFAULT 'local';

-- Create index for OIDC lookups
CREATE INDEX IF NOT EXISTS idx_users_oidc_subject_issuer ON users(oidc_subject, oidc_issuer);
CREATE INDEX IF NOT EXISTS idx_users_auth_provider ON users(auth_provider);

-- Make password_hash optional for OIDC users
ALTER TABLE users ALTER COLUMN password_hash DROP NOT NULL;

-- Add constraint to ensure either password or OIDC fields are provided
ALTER TABLE users ADD CONSTRAINT check_auth_method 
    CHECK (
        (auth_provider = 'local' AND password_hash IS NOT NULL) OR
        (auth_provider = 'oidc' AND oidc_subject IS NOT NULL AND oidc_issuer IS NOT NULL)
    );