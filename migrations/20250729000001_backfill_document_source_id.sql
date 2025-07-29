-- Backfill source_id and source_type for existing documents
-- This migration fixes documents that were ingested before proper source tracking was implemented

-- Update documents that have WebDAV source paths but missing source_id
-- Link them to existing WebDAV sources based on user ownership
UPDATE documents 
SET 
    source_id = sources.id,
    source_type = 'webdav'
FROM sources 
WHERE 
    documents.user_id = sources.user_id 
    AND sources.source_type = 'webdav'
    AND documents.source_metadata->>'source_path' IS NOT NULL
    AND documents.source_id IS NULL;

-- Update documents that have source paths but no source_metadata, likely from older ingestion
-- This handles edge cases where source_path is populated but source_type is not
UPDATE documents 
SET source_type = 'webdav'
WHERE 
    source_path IS NOT NULL 
    AND source_type IS NULL
    AND source_id IN (SELECT id FROM sources WHERE source_type = 'webdav');

-- Add helpful comment explaining the backfill
COMMENT ON COLUMN documents.source_id IS 'References the source that this document was ingested from. Backfilled for existing documents on 2025-07-29.';