-- Add shared_links table for generating shareable document links
CREATE TABLE IF NOT EXISTS shared_links (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token VARCHAR(64) NOT NULL UNIQUE,
    password_hash VARCHAR(255),
    expires_at TIMESTAMPTZ,
    max_views INTEGER,
    view_count INTEGER NOT NULL DEFAULT 0,
    is_revoked BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_shared_links_token ON shared_links(token) WHERE is_revoked = FALSE;
CREATE INDEX idx_shared_links_document_id ON shared_links(document_id);
CREATE INDEX idx_shared_links_created_by ON shared_links(created_by);
