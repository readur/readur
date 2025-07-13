-- Create table to track OCR retry history for audit and analytics
CREATE TABLE IF NOT EXISTS ocr_retry_history (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    retry_reason TEXT,
    previous_status TEXT,
    previous_failure_reason TEXT,
    previous_error TEXT,
    priority INT NOT NULL,
    queue_id UUID,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes for efficient querying
CREATE INDEX idx_ocr_retry_history_document_id ON ocr_retry_history(document_id);
CREATE INDEX idx_ocr_retry_history_user_id ON ocr_retry_history(user_id);
CREATE INDEX idx_ocr_retry_history_created_at ON ocr_retry_history(created_at);

-- Add retry count to documents table if not exists
ALTER TABLE documents 
ADD COLUMN IF NOT EXISTS ocr_retry_count INT DEFAULT 0;

-- Add comment
COMMENT ON TABLE ocr_retry_history IS 'Tracks history of OCR retry attempts for auditing and analytics';
COMMENT ON COLUMN ocr_retry_history.retry_reason IS 'Reason for retry: manual, bulk_retry, scheduled, etc.';
COMMENT ON COLUMN ocr_retry_history.previous_status IS 'OCR status before retry';
COMMENT ON COLUMN ocr_retry_history.previous_failure_reason IS 'Previous failure reason if any';
COMMENT ON COLUMN ocr_retry_history.priority IS 'Priority assigned to the retry in queue';

-- Create view for retry analytics
CREATE OR REPLACE VIEW ocr_retry_analytics AS
SELECT 
    d.id as document_id,
    d.filename,
    d.mime_type,
    d.file_size,
    d.ocr_retry_count,
    d.ocr_status,
    d.ocr_failure_reason,
    COUNT(h.id) as total_retries,
    MAX(h.created_at) as last_retry_at,
    MIN(h.created_at) as first_retry_at
FROM documents d
LEFT JOIN ocr_retry_history h ON d.id = h.document_id
GROUP BY d.id, d.filename, d.mime_type, d.file_size, d.ocr_retry_count, d.ocr_status, d.ocr_failure_reason
HAVING COUNT(h.id) > 0
ORDER BY total_retries DESC;