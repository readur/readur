-- Add WebDAV configuration fields to settings table

ALTER TABLE settings ADD COLUMN IF NOT EXISTS webdav_enabled BOOLEAN DEFAULT FALSE;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS webdav_server_url TEXT;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS webdav_username TEXT;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS webdav_password TEXT;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS webdav_watch_folders TEXT[] DEFAULT ARRAY['/Documents']::TEXT[];
ALTER TABLE settings ADD COLUMN IF NOT EXISTS webdav_file_extensions TEXT[] DEFAULT ARRAY['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt']::TEXT[];
ALTER TABLE settings ADD COLUMN IF NOT EXISTS webdav_auto_sync BOOLEAN DEFAULT FALSE;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS webdav_sync_interval_minutes INTEGER DEFAULT 60;

-- Create table for WebDAV sync state tracking
CREATE TABLE IF NOT EXISTS webdav_sync_state (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    last_sync_at TIMESTAMPTZ,
    sync_cursor TEXT,
    is_running BOOLEAN DEFAULT FALSE,
    files_processed BIGINT DEFAULT 0,
    files_remaining BIGINT DEFAULT 0,
    current_folder TEXT,
    errors TEXT[] DEFAULT ARRAY[]::TEXT[],
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(user_id)
);

-- Create table for tracking WebDAV files
CREATE TABLE IF NOT EXISTS webdav_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    webdav_path TEXT NOT NULL,
    etag TEXT NOT NULL,
    last_modified TIMESTAMPTZ,
    file_size BIGINT,
    mime_type TEXT,
    document_id UUID REFERENCES documents(id) ON DELETE SET NULL,
    sync_status TEXT DEFAULT 'pending', -- pending, processing, completed, failed
    sync_error TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    UNIQUE(user_id, webdav_path)
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_webdav_files_user_id ON webdav_files(user_id);
CREATE INDEX IF NOT EXISTS idx_webdav_files_sync_status ON webdav_files(sync_status);
CREATE INDEX IF NOT EXISTS idx_webdav_files_etag ON webdav_files(etag);
CREATE INDEX IF NOT EXISTS idx_webdav_sync_state_user_id ON webdav_sync_state(user_id);