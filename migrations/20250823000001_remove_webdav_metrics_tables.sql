-- Migration to remove the old WebDAV metrics tables
-- These tables are no longer needed as we've moved to in-memory metrics collection

-- Drop tables in reverse order of dependencies
DROP TABLE IF EXISTS webdav_request_metrics CASCADE;
DROP TABLE IF EXISTS webdav_directory_metrics CASCADE;
DROP TABLE IF EXISTS webdav_sync_sessions CASCADE;

-- Drop any indexes that may have been created
DROP INDEX IF EXISTS idx_webdav_sync_sessions_user_source;
DROP INDEX IF EXISTS idx_webdav_sync_sessions_started_at;
DROP INDEX IF EXISTS idx_webdav_sync_sessions_status;
DROP INDEX IF EXISTS idx_webdav_request_metrics_session;
DROP INDEX IF EXISTS idx_webdav_request_metrics_started_at;
DROP INDEX IF EXISTS idx_webdav_directory_metrics_session;
DROP INDEX IF EXISTS idx_webdav_directory_metrics_path;

-- Drop the enum types if they exist
DROP TYPE IF EXISTS webdav_sync_status CASCADE;
DROP TYPE IF EXISTS webdav_operation_type CASCADE;
DROP TYPE IF EXISTS webdav_request_type CASCADE;
DROP TYPE IF EXISTS webdav_scan_failure_type CASCADE;