-- Normalize existing ETags in webdav_files table to match new normalization format
-- This migration ensures that existing ETag values are normalized to prevent
-- unnecessary re-downloads of unchanged files after the ETag normalization fix

-- Update ETags to remove quotes and W/ prefixes
UPDATE webdav_files 
SET etag = TRIM(BOTH '"' FROM TRIM(LEADING 'W/' FROM etag))
WHERE etag LIKE '"%"' OR etag LIKE 'W/%';

-- Add a comment to document this normalization
COMMENT ON COLUMN webdav_files.etag IS 'Normalized ETag without quotes or W/ prefix (since migration 20250620100020)';