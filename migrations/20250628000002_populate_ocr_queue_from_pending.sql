-- Populate OCR queue with documents that have pending OCR status
-- This migration addresses the issue where documents marked as pending
-- by the confidence backfill migration are not in the processing queue

-- Insert pending documents into OCR queue
INSERT INTO ocr_queue (document_id, priority, file_size, created_at)
SELECT 
    id,
    -- Calculate priority based on file size (smaller files get higher priority)
    CASE 
        WHEN file_size <= 1048576 THEN 10      -- <= 1MB: highest priority
        WHEN file_size <= 5242880 THEN 8       -- 1-5MB: high priority  
        WHEN file_size <= 10485760 THEN 6      -- 5-10MB: medium priority
        WHEN file_size <= 52428800 THEN 4      -- 10-50MB: low priority
        ELSE 2                                  -- > 50MB: lowest priority
    END as priority,
    file_size,
    NOW() as created_at
FROM documents 
WHERE ocr_status = 'pending'
  AND id NOT IN (SELECT document_id FROM ocr_queue)  -- Avoid duplicates
  AND file_path IS NOT NULL                          -- Only queue documents with files
  AND (mime_type LIKE 'image/%' OR mime_type = 'application/pdf' OR mime_type = 'text/plain'); -- Only OCR-able types

-- Log the result
DO $$
DECLARE
    queued_count INTEGER;
BEGIN
    GET DIAGNOSTICS queued_count = ROW_COUNT;
    RAISE NOTICE 'Added % pending documents to OCR queue for processing', queued_count;
END $$;

-- Create helpful index for monitoring
CREATE INDEX IF NOT EXISTS idx_ocr_queue_document_status 
ON ocr_queue(document_id, status, created_at);