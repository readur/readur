-- Add metadata preservation fields to documents table
ALTER TABLE documents
ADD COLUMN original_created_at TIMESTAMPTZ,
ADD COLUMN original_modified_at TIMESTAMPTZ,
ADD COLUMN source_metadata JSONB;

-- Add comment to explain fields
COMMENT ON COLUMN documents.original_created_at IS 'Original file creation timestamp from source system';
COMMENT ON COLUMN documents.original_modified_at IS 'Original file modification timestamp from source system';
COMMENT ON COLUMN documents.source_metadata IS 'Additional metadata from source system (permissions, attributes, EXIF data, etc.)';

-- Create index on source_metadata for efficient JSONB queries
CREATE INDEX idx_documents_source_metadata ON documents USING gin (source_metadata);

-- Note: We cannot reliably populate original_created_at and original_modified_at 
-- for existing documents as we don't have this information stored.
-- These fields will remain NULL for existing documents, which is correct.