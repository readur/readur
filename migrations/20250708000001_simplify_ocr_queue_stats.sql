-- Simplify get_ocr_queue_stats function to avoid CTE structure issues
-- Use a simple SELECT with subqueries instead of CTEs and cross joins

CREATE OR REPLACE FUNCTION get_ocr_queue_stats()
RETURNS TABLE (
    pending_count BIGINT,
    processing_count BIGINT,
    failed_count BIGINT,
    completed_today BIGINT,
    avg_wait_time_minutes DOUBLE PRECISION,
    oldest_pending_minutes DOUBLE PRECISION
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        COUNT(*) FILTER (WHERE status = 'pending') as pending_count,
        COUNT(*) FILTER (WHERE status = 'processing') as processing_count,
        COUNT(*) FILTER (WHERE status = 'failed' AND attempts >= max_attempts) as failed_count,
        -- Get completed_today from documents table instead of ocr_queue
        (SELECT COUNT(*)::BIGINT 
         FROM documents 
         WHERE ocr_status = 'completed'
         AND updated_at >= CURRENT_DATE
         AND updated_at < CURRENT_DATE + INTERVAL '1 day') as completed_today,
        CAST(AVG(EXTRACT(EPOCH FROM (COALESCE(started_at, NOW()) - created_at))/60) FILTER (WHERE status IN ('processing', 'completed')) AS DOUBLE PRECISION) as avg_wait_time_minutes,
        CAST(MAX(EXTRACT(EPOCH FROM (NOW() - created_at))/60) FILTER (WHERE status = 'pending') AS DOUBLE PRECISION) as oldest_pending_minutes
    FROM ocr_queue;
END;
$$ LANGUAGE plpgsql;