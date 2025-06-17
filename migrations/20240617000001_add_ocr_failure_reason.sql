-- Add ocr_failure_reason field to provide detailed error information
-- while keeping ocr_status within valid constraint values

-- Add the ocr_failure_reason column to documents table
ALTER TABLE documents 
ADD COLUMN IF NOT EXISTS ocr_failure_reason TEXT;

-- Create an index for efficient querying of failure reasons
CREATE INDEX IF NOT EXISTS idx_documents_ocr_failure_reason 
ON documents(ocr_failure_reason) 
WHERE ocr_failure_reason IS NOT NULL;

-- Add helpful comments
COMMENT ON COLUMN documents.ocr_failure_reason IS 'Detailed reason for OCR failure when ocr_status is failed - e.g. pdf_font_encoding, timeout, corruption, etc.';

-- Create a view for OCR error analysis
CREATE OR REPLACE VIEW ocr_error_summary AS
SELECT 
    ocr_failure_reason,
    COUNT(*) as error_count,
    COUNT(*) * 100.0 / (SELECT COUNT(*) FROM documents WHERE ocr_status = 'failed') as error_percentage,
    MIN(created_at) as first_occurrence,
    MAX(updated_at) as last_occurrence,
    array_agg(DISTINCT substring(filename from '[^/]*$') ORDER BY substring(filename from '[^/]*$')) as sample_files
FROM documents 
WHERE ocr_status = 'failed' 
  AND ocr_failure_reason IS NOT NULL
GROUP BY ocr_failure_reason
ORDER BY error_count DESC;

-- Grant appropriate permissions (commented out - role may not exist in all environments)
-- GRANT SELECT ON ocr_error_summary TO readur_user;