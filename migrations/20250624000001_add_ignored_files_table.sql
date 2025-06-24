-- Add ignored_files table to track files that have been deleted and should be ignored from their sources
CREATE TABLE ignored_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_hash VARCHAR(64) NOT NULL,
    filename VARCHAR(500) NOT NULL,
    original_filename VARCHAR(500) NOT NULL,
    file_path TEXT NOT NULL,
    file_size BIGINT NOT NULL,
    mime_type VARCHAR(255) NOT NULL,
    source_type VARCHAR(50), -- 'webdav', 's3', 'local', 'upload', etc.
    source_path TEXT, -- Full path from the source
    source_identifier TEXT, -- Additional source context (e.g., WebDAV server, S3 bucket)
    ignored_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    ignored_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reason TEXT, -- Optional reason for ignoring (e.g., "deleted by user", "auto-cleanup")
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient querying
CREATE INDEX idx_ignored_files_file_hash ON ignored_files(file_hash);
CREATE INDEX idx_ignored_files_ignored_by ON ignored_files(ignored_by);
CREATE INDEX idx_ignored_files_source_type ON ignored_files(source_type);
CREATE INDEX idx_ignored_files_ignored_at ON ignored_files(ignored_at);
CREATE INDEX idx_ignored_files_source_path ON ignored_files(source_path);

-- Composite index for checking if a file from a specific source should be ignored
CREATE INDEX idx_ignored_files_lookup ON ignored_files(file_hash, source_type, source_path);