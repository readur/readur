-- Add directory-level ETag tracking for efficient WebDAV sync
-- This optimization allows skipping unchanged directories entirely

CREATE TABLE IF NOT EXISTS webdav_directories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    directory_path TEXT NOT NULL,
    directory_etag TEXT NOT NULL,
    last_scanned_at TIMESTAMPTZ DEFAULT NOW(),
    file_count BIGINT DEFAULT 0,
    total_size_bytes BIGINT DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(user_id, directory_path)
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_webdav_directories_user_id ON webdav_directories(user_id);
CREATE INDEX IF NOT EXISTS idx_webdav_directories_path ON webdav_directories(user_id, directory_path);
CREATE INDEX IF NOT EXISTS idx_webdav_directories_etag ON webdav_directories(directory_etag);
CREATE INDEX IF NOT EXISTS idx_webdav_directories_last_scanned ON webdav_directories(last_scanned_at);