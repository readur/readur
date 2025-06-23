-- Add file_hash field to documents table for efficient duplicate detection
-- This will store SHA256 hash of file content to prevent duplicates

-- Add the file_hash column to documents table
ALTER TABLE documents 
ADD COLUMN IF NOT EXISTS file_hash VARCHAR(64);

-- Create unique index to prevent hash duplicates per user
-- This enforces that each user cannot have duplicate file content
CREATE UNIQUE INDEX IF NOT EXISTS idx_documents_user_file_hash 
ON documents(user_id, file_hash) 
WHERE file_hash IS NOT NULL;

-- Create additional index for efficient hash lookups
CREATE INDEX IF NOT EXISTS idx_documents_file_hash 
ON documents(file_hash) 
WHERE file_hash IS NOT NULL;

-- Add helpful comments
COMMENT ON COLUMN documents.file_hash IS 'SHA256 hash of file content for duplicate detection - prevents same content from being stored multiple times per user';

-- Create a view for duplicate analysis
CREATE OR REPLACE VIEW document_duplicates_analysis AS
SELECT 
    file_hash,
    COUNT(*) as duplicate_count,
    array_agg(DISTINCT user_id ORDER BY user_id) as users_with_duplicates,
    array_agg(filename ORDER BY created_at) as filenames,
    MIN(created_at) as first_upload,
    MAX(created_at) as last_upload,
    SUM(file_size) as total_storage_used
FROM documents 
WHERE file_hash IS NOT NULL
GROUP BY file_hash
HAVING COUNT(*) > 1
ORDER BY duplicate_count DESC, total_storage_used DESC;