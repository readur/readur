-- Migration to move existing failed OCR documents from documents table to failed_documents table
-- This consolidates all failure tracking into a single table

-- First, ensure the failed_documents table exists
-- (This migration depends on 20250628000003_add_failed_documents_table.sql)

-- Move failed OCR documents to failed_documents table
INSERT INTO failed_documents (
    user_id,
    filename,
    original_filename,
    file_path,
    file_size,
    file_hash,
    mime_type,
    content,
    tags,
    ocr_text,
    ocr_confidence,
    ocr_word_count,
    ocr_processing_time_ms,
    failure_reason,
    failure_stage,
    ingestion_source,
    error_message,
    retry_count,
    created_at,
    updated_at
)
SELECT 
    d.user_id,
    d.filename,
    d.original_filename,
    d.file_path,
    d.file_size,
    d.file_hash,
    d.mime_type,
    d.content,
    d.tags,
    d.ocr_text,
    d.ocr_confidence,
    d.ocr_word_count,
    d.ocr_processing_time_ms,
    COALESCE(d.ocr_failure_reason, 'other') as failure_reason,
    'ocr' as failure_stage,
    'migration' as ingestion_source, -- Mark these as migrated from existing system
    d.ocr_error as error_message,
    COALESCE(q.retry_count, 0) as retry_count,
    d.created_at,
    d.updated_at
FROM documents d
LEFT JOIN (
    SELECT document_id, COUNT(*) as retry_count
    FROM ocr_queue 
    WHERE status IN ('failed', 'completed')
    GROUP BY document_id
) q ON d.id = q.document_id
WHERE d.ocr_status = 'failed';

-- Log the migration for audit purposes
INSERT INTO failed_documents (
    user_id,
    filename,
    original_filename,
    failure_reason,
    failure_stage,
    ingestion_source,
    error_message,
    created_at,
    updated_at
) VALUES (
    '00000000-0000-0000-0000-000000000000'::uuid, -- System user ID
    'migration_log',
    'Failed OCR Migration Log',
    'migration_completed',
    'migration',
    'system',
    'Migrated ' || (SELECT COUNT(*) FROM documents WHERE ocr_status = 'failed') || ' failed OCR documents to failed_documents table',
    NOW(),
    NOW()
);

-- Remove failed OCR documents from documents table
-- Note: This uses CASCADE to also clean up related records in ocr_queue table
DELETE FROM documents WHERE ocr_status = 'failed';

-- Update statistics and constraints
ANALYZE documents;
ANALYZE failed_documents;

-- Add comment documenting the migration
COMMENT ON TABLE failed_documents IS 'Tracks all documents that failed at any stage of processing. Consolidated from documents table (OCR failures) and new ingestion failures as of migration 20250628000004.';

-- Create indexes for efficient querying of migrated data
CREATE INDEX IF NOT EXISTS idx_failed_documents_failure_stage_reason ON failed_documents(failure_stage, failure_reason);
CREATE INDEX IF NOT EXISTS idx_failed_documents_ocr_confidence ON failed_documents(ocr_confidence) WHERE ocr_confidence IS NOT NULL;

-- Optional: Create a view for backward compatibility during transition
CREATE OR REPLACE VIEW legacy_failed_ocr_documents AS
SELECT 
    id,
    user_id,
    filename,
    original_filename,
    file_path,
    file_size,
    mime_type,
    tags,
    ocr_text,
    ocr_confidence,
    ocr_word_count,
    ocr_processing_time_ms,
    failure_reason as ocr_failure_reason,
    error_message as ocr_error,
    'failed' as ocr_status,
    retry_count,
    created_at,
    updated_at
FROM failed_documents
WHERE failure_stage = 'ocr';

-- Grant appropriate permissions
-- GRANT SELECT ON legacy_failed_ocr_documents TO readur_user;