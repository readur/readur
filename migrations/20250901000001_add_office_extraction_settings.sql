-- Add office document extraction settings to the settings table
-- This migration adds timeout controls for Office document extraction using XML parsing

-- Add office extraction timeout column (default: 120 seconds)
ALTER TABLE settings 
ADD COLUMN IF NOT EXISTS office_extraction_timeout_seconds INTEGER NOT NULL DEFAULT 120
CHECK (office_extraction_timeout_seconds > 0 AND office_extraction_timeout_seconds <= 600);

-- Add office extraction detailed logging column (default: false for production)
ALTER TABLE settings 
ADD COLUMN IF NOT EXISTS office_extraction_enable_detailed_logging BOOLEAN NOT NULL DEFAULT false;

-- Add comment to document the new columns
COMMENT ON COLUMN settings.office_extraction_timeout_seconds IS 
'Timeout in seconds for office document extraction (1-600 seconds, default: 120)';

COMMENT ON COLUMN settings.office_extraction_enable_detailed_logging IS 
'Enable detailed logging for office document extraction operations (default: false)';

-- The default values are already set in the column definitions above
-- No need to insert default settings as they should be created when users are created

-- ============================================================================
-- TRIGGER RE-OCR FOR .doc AND .docx FILES
-- ============================================================================
-- This section will mark all existing .doc and .docx files for re-processing
-- to take advantage of improved Office document extraction capabilities

-- First, let's count how many documents will be affected
DO $$
DECLARE
    doc_count INTEGER;
    docx_count INTEGER;
    total_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO doc_count 
    FROM documents 
    WHERE LOWER(original_filename) LIKE '%.doc' AND LOWER(original_filename) NOT LIKE '%.docx';
    
    SELECT COUNT(*) INTO docx_count 
    FROM documents 
    WHERE LOWER(original_filename) LIKE '%.docx';
    
    total_count := doc_count + docx_count;
    
    RAISE NOTICE 'Found % .doc files and % .docx files (% total) that will be queued for re-OCR', 
        doc_count, docx_count, total_count;
END $$;

-- Update documents table: Reset OCR status for .doc and .docx files
UPDATE documents 
SET 
    ocr_status = 'pending',
    ocr_text = NULL,
    ocr_confidence = NULL,
    ocr_word_count = NULL,
    ocr_processing_time_ms = NULL,
    ocr_error = NULL,
    ocr_completed_at = NULL,
    updated_at = NOW()
WHERE 
    (LOWER(original_filename) LIKE '%.doc' OR LOWER(original_filename) LIKE '%.docx')
    AND ocr_status != 'pending';  -- Only update if not already pending

-- Add entries to OCR queue for all .doc and .docx files that don't already have pending queue entries
INSERT INTO ocr_queue (document_id, status, priority, attempts, max_attempts, created_at, file_size)
SELECT 
    d.id,
    'pending',
    3,  -- Medium-high priority for office documents
    0,  -- Reset attempts
    3,  -- Standard max attempts
    NOW(),
    d.file_size
FROM documents d
WHERE 
    (LOWER(d.original_filename) LIKE '%.doc' OR LOWER(d.original_filename) LIKE '%.docx')
    AND NOT EXISTS (
        SELECT 1 
        FROM ocr_queue oq 
        WHERE oq.document_id = d.id 
        AND oq.status IN ('pending', 'processing')
    );

-- Log the final count of queued items
DO $$
DECLARE
    queued_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO queued_count 
    FROM ocr_queue oq
    JOIN documents d ON oq.document_id = d.id
    WHERE 
        (LOWER(d.original_filename) LIKE '%.doc' OR LOWER(d.original_filename) LIKE '%.docx')
        AND oq.status = 'pending'
        AND oq.created_at >= NOW() - INTERVAL '1 minute';  -- Recently queued
    
    RAISE NOTICE 'Successfully queued % .doc/.docx files for re-OCR processing', queued_count;
    RAISE NOTICE 'Office document re-OCR migration completed successfully';
END $$;