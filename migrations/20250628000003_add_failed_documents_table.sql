-- Add table to track documents that failed at any stage of processing
-- This provides visibility into documents that failed during: ingestion, validation, OCR, etc.

CREATE TABLE IF NOT EXISTS failed_documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    filename TEXT NOT NULL,
    original_filename TEXT, -- Original name when uploaded (if available)
    original_path TEXT, -- Path where file was located
    file_path TEXT, -- Stored file path (if file was saved before failure)
    file_size BIGINT,
    file_hash VARCHAR(64),
    mime_type TEXT,
    
    -- Document content (if available before failure)
    content TEXT, -- Raw content if extracted
    tags TEXT[], -- Tags that were assigned/detected
    
    -- OCR-related fields (for OCR stage failures)
    ocr_text TEXT, -- Partial OCR text if extracted before failure
    ocr_confidence REAL, -- OCR confidence if calculated
    ocr_word_count INTEGER, -- Word count if calculated
    ocr_processing_time_ms INTEGER, -- Processing time before failure
    
    -- Failure information
    failure_reason TEXT NOT NULL,
    failure_stage TEXT NOT NULL, -- 'ingestion', 'validation', 'ocr', 'storage', etc.
    existing_document_id UUID REFERENCES documents(id) ON DELETE SET NULL,
    ingestion_source TEXT NOT NULL, -- 'batch', 'sync', 'webdav', 'upload', etc.
    error_message TEXT, -- Detailed error information
    
    -- Retry information
    retry_count INTEGER DEFAULT 0,
    last_retry_at TIMESTAMPTZ,
    
    -- Timestamps
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    
    CONSTRAINT check_failure_reason CHECK (failure_reason IN (
        'duplicate_content', 
        'duplicate_filename', 
        'unsupported_format',
        'file_too_large',
        'file_corrupted',
        'access_denied',
        'low_ocr_confidence',
        'ocr_timeout',
        'ocr_memory_limit',
        'pdf_parsing_error',
        'storage_quota_exceeded',
        'network_error',
        'permission_denied',
        'virus_detected',
        'invalid_structure',
        'policy_violation',
        'other'
    )),
    
    CONSTRAINT check_failure_stage CHECK (failure_stage IN (
        'ingestion',
        'validation', 
        'ocr',
        'storage',
        'processing',
        'sync'
    ))
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_failed_documents_user_id ON failed_documents(user_id);
CREATE INDEX IF NOT EXISTS idx_failed_documents_created_at ON failed_documents(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_failed_documents_failure_reason ON failed_documents(failure_reason);
CREATE INDEX IF NOT EXISTS idx_failed_documents_failure_stage ON failed_documents(failure_stage);
CREATE INDEX IF NOT EXISTS idx_failed_documents_ingestion_source ON failed_documents(ingestion_source);
CREATE INDEX IF NOT EXISTS idx_failed_documents_file_hash ON failed_documents(file_hash) WHERE file_hash IS NOT NULL;

-- Add comments for documentation
COMMENT ON TABLE failed_documents IS 'Tracks documents that failed at any stage of processing (ingestion, validation, OCR, etc.)';
COMMENT ON COLUMN failed_documents.failure_reason IS 'Specific reason why the document failed';
COMMENT ON COLUMN failed_documents.failure_stage IS 'Stage at which the document failed (ingestion, validation, ocr, etc.)';
COMMENT ON COLUMN failed_documents.existing_document_id IS 'Reference to existing document if failed due to duplicate content';
COMMENT ON COLUMN failed_documents.ingestion_source IS 'Source of the ingestion attempt (batch, sync, webdav, upload, etc.)';
COMMENT ON COLUMN failed_documents.error_message IS 'Detailed error message for troubleshooting';

-- Create a view for failed documents summary by reason and stage
CREATE OR REPLACE VIEW failed_documents_summary AS
SELECT 
    failure_reason,
    failure_stage,
    ingestion_source,
    COUNT(*) as document_count,
    SUM(file_size) as total_size,
    AVG(file_size) as avg_size,
    MIN(created_at) as first_occurrence,
    MAX(created_at) as last_occurrence
FROM failed_documents 
GROUP BY failure_reason, failure_stage, ingestion_source
ORDER BY document_count DESC;

-- Grant appropriate permissions
-- GRANT SELECT, INSERT ON failed_documents TO readur_user;
-- GRANT SELECT ON failed_documents_summary TO readur_user;