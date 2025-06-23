-- Create sources table to support multiple document sources per user
CREATE TABLE IF NOT EXISTS sources (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    source_type TEXT NOT NULL, -- 'webdav', 'local_folder', 's3', etc.
    enabled BOOLEAN DEFAULT TRUE,
    
    -- Configuration (JSON to allow flexibility for different source types)
    config JSONB NOT NULL DEFAULT '{}',
    
    -- Status tracking
    status TEXT DEFAULT 'idle', -- 'idle', 'syncing', 'error'
    last_sync_at TIMESTAMPTZ,
    last_error TEXT,
    last_error_at TIMESTAMPTZ,
    
    -- Statistics
    total_files_synced BIGINT DEFAULT 0,
    total_files_pending BIGINT DEFAULT 0,
    total_size_bytes BIGINT DEFAULT 0,
    
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(user_id, name)
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_sources_user_id ON sources(user_id);
CREATE INDEX IF NOT EXISTS idx_sources_source_type ON sources(source_type);
CREATE INDEX IF NOT EXISTS idx_sources_status ON sources(status);

-- Update documents table to link to sources
ALTER TABLE documents ADD COLUMN IF NOT EXISTS source_id UUID REFERENCES sources(id) ON DELETE SET NULL;
CREATE INDEX IF NOT EXISTS idx_documents_source_id ON documents(source_id);

-- Update webdav_files table to link to sources instead of users directly
ALTER TABLE webdav_files ADD COLUMN IF NOT EXISTS source_id UUID REFERENCES sources(id) ON DELETE CASCADE;

-- Migrate existing WebDAV settings to sources table
INSERT INTO sources (user_id, name, source_type, enabled, config, created_at, updated_at)
SELECT 
    s.user_id,
    'WebDAV Server' as name,
    'webdav' as source_type,
    s.webdav_enabled as enabled,
    jsonb_build_object(
        'server_url', s.webdav_server_url,
        'username', s.webdav_username,
        'password', s.webdav_password,
        'watch_folders', s.webdav_watch_folders,
        'file_extensions', s.webdav_file_extensions,
        'auto_sync', s.webdav_auto_sync,
        'sync_interval_minutes', s.webdav_sync_interval_minutes
    ) as config,
    NOW() as created_at,
    NOW() as updated_at
FROM settings s
WHERE s.webdav_enabled = TRUE 
  AND s.webdav_server_url IS NOT NULL 
  AND s.webdav_username IS NOT NULL;

-- Update webdav_files to link to the newly created sources
UPDATE webdav_files wf
SET source_id = s.id
FROM sources s
WHERE wf.user_id = s.user_id 
  AND s.source_type = 'webdav';

-- Create a function to update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_sources_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create trigger to auto-update updated_at
CREATE TRIGGER sources_updated_at_trigger
BEFORE UPDATE ON sources
FOR EACH ROW
EXECUTE FUNCTION update_sources_updated_at();

-- Note: We're keeping the webdav fields in settings table for now to ensure backward compatibility
-- They will be removed in a future migration after ensuring all code is updated