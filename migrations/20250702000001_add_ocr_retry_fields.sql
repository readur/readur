-- Add OCR retry tracking fields to documents table
-- These fields were added to the Document struct but missing from the database schema

ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_retry_count INTEGER DEFAULT 0;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_failure_reason TEXT DEFAULT NULL;

-- Add helpful comments
COMMENT ON COLUMN documents.ocr_retry_count IS 'Number of times OCR processing has been retried for this document';
COMMENT ON COLUMN documents.ocr_failure_reason IS 'Reason for the most recent OCR failure, if any';