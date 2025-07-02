-- Database Guardrails Migration
-- Adds constraints, indexes, and triggers to prevent data corruption

-- 1. Add constraints to prevent invalid OCR states
ALTER TABLE documents ADD CONSTRAINT check_ocr_status 
CHECK (ocr_status IN ('pending', 'processing', 'completed', 'failed'));

-- 2. Add constraint to ensure OCR confidence is valid
ALTER TABLE documents ADD CONSTRAINT check_ocr_confidence 
CHECK (ocr_confidence IS NULL OR (ocr_confidence >= 0 AND ocr_confidence <= 100));

-- 3. Add constraint to ensure word count is non-negative
ALTER TABLE documents ADD CONSTRAINT check_ocr_word_count 
CHECK (ocr_word_count IS NULL OR ocr_word_count >= 0);

-- 4. Add constraint to ensure processing time is non-negative  
ALTER TABLE documents ADD CONSTRAINT check_ocr_processing_time 
CHECK (ocr_processing_time_ms IS NULL OR ocr_processing_time_ms >= 0);

-- 5. Create partial index for pending OCR documents (faster queue operations)
CREATE INDEX IF NOT EXISTS idx_documents_pending_ocr 
ON documents (created_at) 
WHERE ocr_status = 'pending';

-- 6. Create partial index for processing OCR documents (monitoring stuck jobs)
CREATE INDEX IF NOT EXISTS idx_documents_processing_ocr 
ON documents (updated_at) 
WHERE ocr_status = 'processing';

-- 7. Add foreign key constraint with CASCADE to maintain referential integrity
ALTER TABLE ocr_queue 
ADD CONSTRAINT fk_ocr_queue_document_id 
FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE;

-- 8. Add constraint to OCR queue status
ALTER TABLE ocr_queue ADD CONSTRAINT check_queue_status 
CHECK (status IN ('pending', 'processing', 'completed', 'failed'));

-- 9. Add constraint to ensure attempts don't exceed max_attempts
ALTER TABLE ocr_queue ADD CONSTRAINT check_attempts_limit 
CHECK (attempts <= max_attempts);

-- 10. Add constraint to ensure priority is within reasonable range
ALTER TABLE ocr_queue ADD CONSTRAINT check_priority_range 
CHECK (priority >= 0 AND priority <= 1000);

-- 11. Add unique constraint to prevent duplicate queue entries
CREATE UNIQUE INDEX IF NOT EXISTS idx_ocr_queue_unique_pending_document 
ON ocr_queue (document_id) 
WHERE status IN ('pending', 'processing');

-- 12. Create function to validate OCR data consistency
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

-- 13. Create trigger to enforce OCR consistency
CREATE TRIGGER trigger_validate_ocr_consistency
    BEFORE UPDATE ON documents
    FOR EACH ROW
    WHEN (OLD.ocr_status IS DISTINCT FROM NEW.ocr_status OR 
          OLD.ocr_text IS DISTINCT FROM NEW.ocr_text)
    EXECUTE FUNCTION validate_ocr_consistency();

-- 14. Create function to automatically clean up completed queue items
CREATE OR REPLACE FUNCTION cleanup_completed_ocr_queue()
RETURNS TRIGGER AS $$
BEGIN
    -- Remove queue item when document OCR is completed
    IF NEW.ocr_status = 'completed' AND OLD.ocr_status != 'completed' THEN
        DELETE FROM ocr_queue WHERE document_id = NEW.id;
        RAISE NOTICE 'Removed completed OCR queue item for document %', NEW.id;
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 15. Create trigger for automatic queue cleanup
CREATE TRIGGER trigger_cleanup_completed_ocr_queue
    AFTER UPDATE ON documents
    FOR EACH ROW
    WHEN (NEW.ocr_status = 'completed' AND OLD.ocr_status != 'completed')
    EXECUTE FUNCTION cleanup_completed_ocr_queue();

-- 16. Create function to prevent orphaned queue items
CREATE OR REPLACE FUNCTION prevent_orphaned_queue_items()
RETURNS TRIGGER AS $$
BEGIN
    -- Ensure document exists before creating queue item
    IF NOT EXISTS (SELECT 1 FROM documents WHERE id = NEW.document_id) THEN
        RAISE EXCEPTION 'Cannot create OCR queue item for non-existent document %', NEW.document_id;
    END IF;
    
    -- Prevent duplicate queue items for the same document
    IF EXISTS (
        SELECT 1 FROM ocr_queue 
        WHERE document_id = NEW.document_id 
          AND status IN ('pending', 'processing')
          AND id != COALESCE(NEW.id, '00000000-0000-0000-0000-000000000000'::uuid)
    ) THEN
        RAISE EXCEPTION 'OCR queue item already exists for document %', NEW.document_id;
    END IF;
    
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- 17. Create trigger to prevent orphaned queue items
CREATE TRIGGER trigger_prevent_orphaned_queue_items
    BEFORE INSERT OR UPDATE ON ocr_queue
    FOR EACH ROW
    EXECUTE FUNCTION prevent_orphaned_queue_items();

-- 18. Create function for monitoring stuck OCR jobs
CREATE OR REPLACE FUNCTION find_stuck_ocr_jobs(stuck_threshold_minutes INTEGER DEFAULT 30)
RETURNS TABLE (
    document_id UUID,
    filename TEXT,
    worker_id TEXT,
    started_at TIMESTAMPTZ,
    minutes_stuck INTEGER
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        d.id,
        d.filename,
        q.worker_id,
        q.started_at,
        EXTRACT(EPOCH FROM (NOW() - q.started_at))::INTEGER / 60 as minutes_stuck
    FROM documents d
    JOIN ocr_queue q ON d.id = q.document_id
    WHERE d.ocr_status = 'processing'
      AND q.status = 'processing'
      AND q.started_at < NOW() - (stuck_threshold_minutes || ' minutes')::INTERVAL
    ORDER BY q.started_at ASC;
END;
$$ LANGUAGE plpgsql;

-- 19. Create function to reset stuck OCR jobs
CREATE OR REPLACE FUNCTION reset_stuck_ocr_jobs(stuck_threshold_minutes INTEGER DEFAULT 30)
RETURNS INTEGER AS $$
DECLARE
    reset_count INTEGER;
BEGIN
    -- Reset documents stuck in processing
    UPDATE documents 
    SET ocr_status = 'pending', updated_at = NOW()
    WHERE ocr_status = 'processing'
      AND updated_at < NOW() - (stuck_threshold_minutes || ' minutes')::INTERVAL;
    
    GET DIAGNOSTICS reset_count = ROW_COUNT;
    
    -- Reset corresponding queue items
    UPDATE ocr_queue
    SET status = 'pending', 
        worker_id = NULL, 
        started_at = NULL,
        error_message = 'Reset due to timeout'
    WHERE status = 'processing'
      AND started_at < NOW() - (stuck_threshold_minutes || ' minutes')::INTERVAL;
    
    RAISE NOTICE 'Reset % stuck OCR jobs', reset_count;
    RETURN reset_count;
END;
$$ LANGUAGE plpgsql;

-- 20. Create materialized view for OCR statistics (refreshed periodically)
CREATE MATERIALIZED VIEW ocr_stats AS
SELECT 
    COUNT(*) FILTER (WHERE ocr_status = 'pending') as pending_count,
    COUNT(*) FILTER (WHERE ocr_status = 'processing') as processing_count,
    COUNT(*) FILTER (WHERE ocr_status = 'completed') as completed_count,
    COUNT(*) FILTER (WHERE ocr_status = 'failed') as failed_count,
    AVG(ocr_confidence) FILTER (WHERE ocr_status = 'completed') as avg_confidence,
    AVG(ocr_word_count) FILTER (WHERE ocr_status = 'completed') as avg_word_count,
    AVG(ocr_processing_time_ms) FILTER (WHERE ocr_status = 'completed') as avg_processing_time_ms,
    COUNT(*) FILTER (WHERE ocr_status = 'processing' AND updated_at < NOW() - INTERVAL '30 minutes') as stuck_count,
    NOW() as last_updated
FROM documents;

-- Create index on the materialized view
CREATE UNIQUE INDEX IF NOT EXISTS idx_ocr_stats_unique ON ocr_stats (last_updated);

-- 21. Create function to refresh OCR stats
CREATE OR REPLACE FUNCTION refresh_ocr_stats()
RETURNS VOID AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY ocr_stats;
END;
$$ LANGUAGE plpgsql;

-- Add comments for documentation
COMMENT ON CONSTRAINT check_ocr_status ON documents IS 'Ensures OCR status is one of the valid values';
COMMENT ON CONSTRAINT check_ocr_confidence ON documents IS 'Ensures OCR confidence is between 0 and 100';
COMMENT ON FUNCTION validate_ocr_consistency() IS 'Validates OCR data consistency during updates';
COMMENT ON FUNCTION cleanup_completed_ocr_queue() IS 'Automatically removes queue items when OCR completes';
COMMENT ON FUNCTION find_stuck_ocr_jobs(INTEGER) IS 'Identifies OCR jobs that have been processing too long';
COMMENT ON FUNCTION reset_stuck_ocr_jobs(INTEGER) IS 'Resets OCR jobs that appear to be stuck';
COMMENT ON MATERIALIZED VIEW ocr_stats IS 'Aggregated statistics about OCR processing status';