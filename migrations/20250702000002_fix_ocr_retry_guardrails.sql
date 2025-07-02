-- Fix OCR retry functionality by allowing completed -> pending transitions
-- This addresses the issue where retry operations were blocked by database guardrails

-- Update the OCR consistency validation function to allow retry operations
CREATE OR REPLACE FUNCTION validate_ocr_consistency()
RETURNS TRIGGER AS $$
BEGIN
    -- Allow OCR retry operations: completed -> pending is allowed for retry functionality
    -- Prevent other modifications to completed OCR data
    IF OLD.ocr_status = 'completed' AND NEW.ocr_status != 'completed' AND NEW.ocr_status != 'pending' THEN
        RAISE EXCEPTION 'Cannot modify completed OCR data for document %. Only retry (pending) is allowed.', OLD.id;
    END IF;
    
    -- Ensure OCR text and metadata consistency
    IF NEW.ocr_status = 'completed' AND NEW.ocr_text IS NOT NULL THEN
        -- Check that confidence and word count are reasonable
        IF NEW.ocr_confidence IS NULL OR NEW.ocr_word_count IS NULL THEN
            RAISE WARNING 'OCR completed but missing confidence or word count for document %', NEW.id;
        END IF;
        
        -- Validate word count roughly matches text length
        IF NEW.ocr_word_count > 0 AND length(NEW.ocr_text) < NEW.ocr_word_count THEN
            RAISE WARNING 'OCR word count (%) seems too high for text length (%) in document %', 
                NEW.ocr_word_count, length(NEW.ocr_text), NEW.id;
        END IF;
    END IF;
    
    -- Set completion timestamp when status changes to completed
    IF OLD.ocr_status != 'completed' AND NEW.ocr_status = 'completed' THEN
        NEW.ocr_completed_at = NOW();
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Add comment to document the change
COMMENT ON FUNCTION validate_ocr_consistency() IS 'Validates OCR data consistency during updates. Updated to allow retry operations (completed -> pending transitions).';