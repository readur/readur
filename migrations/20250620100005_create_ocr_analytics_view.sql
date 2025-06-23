CREATE OR REPLACE VIEW ocr_analytics AS
SELECT 
    DATE(created_at) as date,
    COUNT(*) as total_documents,
    COUNT(ocr_text) as documents_with_ocr,
    COUNT(ocr_confidence) as documents_with_confidence,
    AVG(ocr_confidence) as avg_confidence,
    MIN(ocr_confidence) as min_confidence,
    MAX(ocr_confidence) as max_confidence,
    AVG(ocr_word_count) as avg_word_count,
    SUM(ocr_word_count) as total_words_extracted,
    AVG(ocr_processing_time_ms) as avg_processing_time_ms,
    COUNT(*) FILTER (WHERE ocr_confidence < 50) as low_confidence_count,
    COUNT(*) FILTER (WHERE ocr_confidence >= 80) as high_confidence_count,
    COUNT(*) FILTER (WHERE ocr_status = 'failed') as failed_ocr_count
FROM documents 
WHERE created_at >= CURRENT_DATE - INTERVAL '30 days'
GROUP BY DATE(created_at)
ORDER BY date DESC;