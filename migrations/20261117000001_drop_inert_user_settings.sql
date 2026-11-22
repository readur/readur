-- Drop per-user settings that were stored and shown but changed nothing:
--   concurrent_ocr_jobs   the OCR worker pool is server-wide (CONCURRENT_OCR_JOBS)
--   cpu_priority, memory_limit_mb
--                         no OCR process was ever niced or limited by them
--   auto_detect_language_combination
--                         every preferred language is always combined
--   search_results_per_page, search_snippet_length
--                         each search request carries its own limit and snippet length
--   retention_days, enable_auto_cleanup, enable_compression
--                         nothing expired, cleaned up or compressed documents
--   allowed_file_types    nothing read it; file types are filtered by the server-wide
--                         ALLOWED_FILE_TYPES
--   office_extraction_enable_detailed_logging
--                         extraction logging follows RUST_LOG
ALTER TABLE settings
    DROP COLUMN IF EXISTS concurrent_ocr_jobs,
    DROP COLUMN IF EXISTS cpu_priority,
    DROP COLUMN IF EXISTS memory_limit_mb,
    DROP COLUMN IF EXISTS auto_detect_language_combination,
    DROP COLUMN IF EXISTS search_results_per_page,
    DROP COLUMN IF EXISTS search_snippet_length,
    DROP COLUMN IF EXISTS retention_days,
    DROP COLUMN IF EXISTS enable_auto_cleanup,
    DROP COLUMN IF EXISTS enable_compression,
    DROP COLUMN IF EXISTS allowed_file_types,
    DROP COLUMN IF EXISTS office_extraction_enable_detailed_logging;

-- max_file_size_mb is now enforced at upload. Rows still holding the old
-- column default (50) never chose a limit, so they follow the server's
-- MAX_FILE_SIZE_MB instead of being newly capped at 50 MB; NULL means that.
ALTER TABLE settings ALTER COLUMN max_file_size_mb DROP DEFAULT;
ALTER TABLE settings ALTER COLUMN max_file_size_mb DROP NOT NULL;
UPDATE settings SET max_file_size_mb = NULL WHERE max_file_size_mb = 50;
