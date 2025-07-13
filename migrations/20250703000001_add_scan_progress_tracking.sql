-- Add scan progress tracking for crash recovery
-- This allows resuming interrupted scans after server restarts

ALTER TABLE webdav_directories 
ADD COLUMN IF NOT EXISTS scan_in_progress BOOLEAN DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS scan_started_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS scan_error TEXT;

-- Create index for finding incomplete scans
CREATE INDEX IF NOT EXISTS idx_webdav_directories_scan_progress 
ON webdav_directories(user_id, scan_in_progress) 
WHERE scan_in_progress = TRUE;

-- Create index for finding scans that have been running too long (possible crashes)
CREATE INDEX IF NOT EXISTS idx_webdav_directories_stale_scans 
ON webdav_directories(scan_started_at) 
WHERE scan_in_progress = TRUE;