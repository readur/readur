-- Backfill OCR confidence scores for existing documents
-- Since OCR confidence was previously hardcoded to 85%, we need to recalculate
-- actual confidence for documents that currently have this placeholder value

-- First, let's identify documents that likely have placeholder confidence
-- (85% exactly, which was the hardcoded value)
CREATE TEMP TABLE documents_to_update AS
SELECT id, ocr_text, ocr_status 
FROM documents 
WHERE ocr_confidence = 85.0 
  AND ocr_status = 'completed' 
  AND ocr_text IS NOT NULL 
  AND length(trim(ocr_text)) > 0;

-- For now, we'll estimate confidence based on text quality metrics
-- This is a rough approximation until we can re-run OCR with actual confidence
UPDATE documents 
SET ocr_confidence = CASE
    -- High quality text: good length, reasonable character distribution
    WHEN length(trim(ocr_text)) > 1000 
         AND (length(ocr_text) - length(replace(replace(ocr_text, ' ', ''), char(10), ''))) * 100.0 / length(ocr_text) > 10.0  -- > 10% whitespace
         AND length(replace(replace(replace(ocr_text, ' ', ''), char(10), ''), char(13), '')) * 100.0 / length(ocr_text) > 70.0  -- > 70% non-whitespace chars
    THEN 90.0 + (random() * 8.0)  -- 90-98%
    
    -- Medium quality text: decent length, some structure
    WHEN length(trim(ocr_text)) > 100 
         AND (length(ocr_text) - length(replace(replace(ocr_text, ' ', ''), char(10), ''))) * 100.0 / length(ocr_text) > 5.0   -- > 5% whitespace
         AND length(replace(replace(replace(ocr_text, ' ', ''), char(10), ''), char(13), '')) * 100.0 / length(ocr_text) > 50.0  -- > 50% non-whitespace chars
    THEN 70.0 + (random() * 15.0)  -- 70-85%
    
    -- Low quality text: short or poor structure
    WHEN length(trim(ocr_text)) > 10
         AND length(replace(replace(replace(ocr_text, ' ', ''), char(10), ''), char(13), '')) * 100.0 / length(ocr_text) > 30.0  -- > 30% non-whitespace chars
    THEN 40.0 + (random() * 25.0)  -- 40-65%
    
    -- Very poor quality: very short or mostly garbage
    ELSE 20.0 + (random() * 15.0)  -- 20-35%
END
WHERE id IN (SELECT id FROM documents_to_update);

-- Add a comment explaining what we did
COMMENT ON COLUMN documents.ocr_confidence IS 'OCR confidence percentage (0-100). Values may be estimated for documents processed before real confidence calculation was implemented.';

-- Log the update
DO $$
DECLARE
    updated_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO updated_count FROM documents_to_update;
    RAISE NOTICE 'Backfilled OCR confidence for % documents that had placeholder 85%% confidence', updated_count;
END $$;

-- Clean up
DROP TABLE documents_to_update;

-- Create an index to help with confidence-based queries
CREATE INDEX IF NOT EXISTS idx_documents_ocr_confidence_range 
ON documents(ocr_confidence) 
WHERE ocr_confidence IS NOT NULL;