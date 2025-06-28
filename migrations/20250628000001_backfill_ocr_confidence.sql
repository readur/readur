-- Re-queue documents with placeholder OCR confidence for reprocessing
-- Since OCR confidence was previously hardcoded to 85%, we need to reprocess
-- these documents to get accurate confidence scores

-- Temporarily disable the OCR consistency trigger to allow this migration
ALTER TABLE documents DISABLE TRIGGER trigger_validate_ocr_consistency;

-- Mark documents with exactly 85% confidence as pending OCR reprocessing
UPDATE documents 
SET ocr_status = 'pending',
    ocr_confidence = NULL,
    ocr_error = NULL,
    updated_at = CURRENT_TIMESTAMP
WHERE ocr_confidence = 85.0 
  AND ocr_status = 'completed'
  AND ocr_text IS NOT NULL;

-- Re-enable the OCR consistency trigger
ALTER TABLE documents ENABLE TRIGGER trigger_validate_ocr_consistency;

-- Add a comment explaining what we did
COMMENT ON COLUMN documents.ocr_confidence IS 'OCR confidence percentage (0-100) from Tesseract. Documents with NULL confidence and pending status will be reprocessed.';

-- Log the update
DO $$
DECLARE
    updated_count INTEGER;
BEGIN
    GET DIAGNOSTICS updated_count = ROW_COUNT;
    RAISE NOTICE 'Marked % documents with placeholder 85%% confidence for OCR reprocessing', updated_count;
END $$;

-- Create an index to help with confidence-based queries
CREATE INDEX IF NOT EXISTS idx_documents_ocr_confidence_range 
ON documents(ocr_confidence) 
WHERE ocr_confidence IS NOT NULL;

-- Create an index to help the OCR queue find pending documents efficiently
CREATE INDEX IF NOT EXISTS idx_documents_ocr_pending 
ON documents(created_at) 
WHERE ocr_status = 'pending';