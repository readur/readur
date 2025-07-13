-- Add remaining dedicated metadata fields to documents table
-- These fields extract commonly used metadata from source_metadata JSON 
-- into dedicated columns for better querying and indexing

-- Add source path (original file location from source system)
ALTER TABLE documents 
ADD COLUMN IF NOT EXISTS source_path TEXT;

-- Add source type (e.g., 'web_upload', 'filesystem', 'webdav', 's3')
ALTER TABLE documents 
ADD COLUMN IF NOT EXISTS source_type TEXT;

-- Add file permissions (Unix mode bits from source system)
ALTER TABLE documents 
ADD COLUMN IF NOT EXISTS file_permissions INTEGER;

-- Add file owner (username or uid from source system)
ALTER TABLE documents 
ADD COLUMN IF NOT EXISTS file_owner TEXT;

-- Add file group (groupname or gid from source system)
ALTER TABLE documents 
ADD COLUMN IF NOT EXISTS file_group TEXT;

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_documents_source_path ON documents(source_path) 
WHERE source_path IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_documents_source_type ON documents(source_type) 
WHERE source_type IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_documents_file_permissions ON documents(file_permissions) 
WHERE file_permissions IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_documents_file_owner ON documents(file_owner) 
WHERE file_owner IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_documents_file_group ON documents(file_group) 
WHERE file_group IS NOT NULL;

-- Add helpful comments
COMMENT ON COLUMN documents.source_path IS 'Original path where the file was located in the source system';
COMMENT ON COLUMN documents.source_type IS 'Type of source where file was ingested from (web_upload, filesystem, webdav, s3, etc.)';
COMMENT ON COLUMN documents.file_permissions IS 'File permissions from source system (Unix mode bits)';
COMMENT ON COLUMN documents.file_owner IS 'File owner from source system (username or uid)';
COMMENT ON COLUMN documents.file_group IS 'File group from source system (groupname or gid)';