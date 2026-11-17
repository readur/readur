-- One pending/processing OCR job per document is enforced by the partial
-- unique index idx_ocr_queue_unique_pending_document. The guardrail trigger
-- (20250620100013) also checked for duplicates, but by RAISE EXCEPTION in a
-- BEFORE trigger, which fires before ON CONFLICT can arbitrate: an
-- "enqueue unless already queued" insert failed instead of doing nothing,
-- and concurrent OCR retries returned errors. Keep the trigger's orphan
-- check; leave duplicates to the index (unique_violation, which callers can
-- handle with ON CONFLICT).
CREATE OR REPLACE FUNCTION prevent_orphaned_queue_items()
RETURNS TRIGGER AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM documents WHERE id = NEW.document_id) THEN
        RAISE EXCEPTION 'Cannot create OCR queue item for non-existent document %', NEW.document_id;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
