-- WebDAV Metrics Collection System
-- This migration adds tables for tracking detailed WebDAV sync performance metrics

-- Create enum for WebDAV operation types
-- Use DO block to handle existing type gracefully
DO $$ BEGIN
    CREATE TYPE webdav_operation_type AS ENUM (
        'discovery',           -- Directory/file discovery operations
        'download',           -- File download operations
        'metadata_fetch',     -- Getting file metadata (properties)
        'connection_test',    -- Testing connection/authentication
        'validation',         -- Directory validation operations
        'full_sync'           -- Complete sync session
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Create enum for WebDAV request types (HTTP methods)
-- Use DO block to handle existing type gracefully
DO $$ BEGIN
    CREATE TYPE webdav_request_type AS ENUM (
        'PROPFIND',
        'GET',
        'HEAD',
        'OPTIONS',
        'POST',
        'PUT',
        'DELETE'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Table for tracking overall WebDAV sync sessions
CREATE TABLE IF NOT EXISTS webdav_sync_sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source_id UUID REFERENCES sources(id) ON DELETE CASCADE,
    
    -- Session timing
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    duration_ms BIGINT, -- Total session duration in milliseconds
    
    -- Session scope and configuration
    sync_type TEXT NOT NULL, -- 'full', 'incremental', 'validation', etc.
    root_path TEXT NOT NULL, -- Starting path for the sync
    max_depth INTEGER, -- Maximum directory depth scanned
    
    -- Discovery metrics
    directories_discovered INTEGER NOT NULL DEFAULT 0,
    directories_processed INTEGER NOT NULL DEFAULT 0,
    files_discovered INTEGER NOT NULL DEFAULT 0,
    files_processed INTEGER NOT NULL DEFAULT 0,
    
    -- Size and performance metrics
    total_bytes_discovered BIGINT NOT NULL DEFAULT 0,
    total_bytes_processed BIGINT NOT NULL DEFAULT 0,
    avg_file_size_bytes BIGINT,
    processing_rate_files_per_sec FLOAT8,
    
    -- Request statistics
    total_http_requests INTEGER NOT NULL DEFAULT 0,
    successful_requests INTEGER NOT NULL DEFAULT 0,
    failed_requests INTEGER NOT NULL DEFAULT 0,
    retry_attempts INTEGER NOT NULL DEFAULT 0,
    
    -- Error handling
    directories_skipped INTEGER NOT NULL DEFAULT 0,
    files_skipped INTEGER NOT NULL DEFAULT 0,
    skip_reasons JSONB, -- JSON object with skip reason counts
    
    -- Final status
    status TEXT NOT NULL DEFAULT 'in_progress', -- 'completed', 'failed', 'cancelled'
    final_error_message TEXT,
    
    -- Performance insights
    slowest_operation_ms BIGINT,
    slowest_operation_path TEXT,
    network_time_ms BIGINT, -- Time spent on network operations
    processing_time_ms BIGINT, -- Time spent on local processing
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Table for tracking per-directory scan metrics
CREATE TABLE IF NOT EXISTS webdav_directory_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES webdav_sync_sessions(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source_id UUID REFERENCES sources(id) ON DELETE CASCADE,
    
    -- Directory identification
    directory_path TEXT NOT NULL,
    directory_depth INTEGER NOT NULL DEFAULT 0,
    parent_directory_path TEXT,
    
    -- Timing metrics
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    scan_duration_ms BIGINT,
    
    -- Discovery results
    files_found INTEGER NOT NULL DEFAULT 0,
    subdirectories_found INTEGER NOT NULL DEFAULT 0,
    total_size_bytes BIGINT NOT NULL DEFAULT 0,
    
    -- Processing results
    files_processed INTEGER NOT NULL DEFAULT 0,
    files_skipped INTEGER NOT NULL DEFAULT 0,
    files_failed INTEGER NOT NULL DEFAULT 0,
    
    -- Request details
    http_requests_made INTEGER NOT NULL DEFAULT 0,
    propfind_requests INTEGER NOT NULL DEFAULT 0,
    get_requests INTEGER NOT NULL DEFAULT 0,
    
    -- Error information
    errors_encountered INTEGER NOT NULL DEFAULT 0,
    error_types JSONB, -- JSON array of error types encountered
    warnings_count INTEGER NOT NULL DEFAULT 0,
    
    -- Performance characteristics
    avg_response_time_ms FLOAT8,
    slowest_request_ms BIGINT,
    fastest_request_ms BIGINT,
    
    -- ETag and caching
    etag_matches INTEGER NOT NULL DEFAULT 0,
    etag_mismatches INTEGER NOT NULL DEFAULT 0,
    cache_hits INTEGER NOT NULL DEFAULT 0,
    cache_misses INTEGER NOT NULL DEFAULT 0,
    
    -- Final status
    status TEXT NOT NULL DEFAULT 'completed', -- 'completed', 'failed', 'skipped'
    skip_reason TEXT,
    error_message TEXT,
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Table for tracking individual HTTP request metrics
CREATE TABLE IF NOT EXISTS webdav_request_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID REFERENCES webdav_sync_sessions(id) ON DELETE CASCADE,
    directory_metric_id UUID REFERENCES webdav_directory_metrics(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source_id UUID REFERENCES sources(id) ON DELETE CASCADE,
    
    -- Request identification
    request_type webdav_request_type NOT NULL,
    operation_type webdav_operation_type NOT NULL,
    target_path TEXT NOT NULL,
    
    -- Timing
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    duration_ms BIGINT NOT NULL,
    
    -- Request details
    request_size_bytes BIGINT,
    response_size_bytes BIGINT,
    http_status_code INTEGER,
    
    -- Performance metrics
    dns_lookup_ms BIGINT,
    tcp_connect_ms BIGINT,
    tls_handshake_ms BIGINT,
    time_to_first_byte_ms BIGINT,
    
    -- Success/failure tracking
    success BOOLEAN NOT NULL,
    retry_attempt INTEGER NOT NULL DEFAULT 0,
    error_type TEXT,
    error_message TEXT,
    
    -- Server response details
    server_header TEXT,
    dav_header TEXT,
    etag_value TEXT,
    last_modified TIMESTAMPTZ,
    content_type TEXT,
    
    -- Network context
    remote_ip TEXT,
    user_agent TEXT,
    
    -- Metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_webdav_sync_sessions_user_id ON webdav_sync_sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_webdav_sync_sessions_source_id ON webdav_sync_sessions(source_id);
CREATE INDEX IF NOT EXISTS idx_webdav_sync_sessions_started_at ON webdav_sync_sessions(started_at);
CREATE INDEX IF NOT EXISTS idx_webdav_sync_sessions_status ON webdav_sync_sessions(status);

-- JSON indexes for skip_reasons queries
CREATE INDEX IF NOT EXISTS idx_webdav_sync_sessions_skip_reasons_gin ON webdav_sync_sessions USING gin(skip_reasons);

-- Specific indexes for common skip_reasons queries
CREATE INDEX IF NOT EXISTS idx_webdav_sync_sessions_skip_reasons_keys ON webdav_sync_sessions 
    USING gin((skip_reasons -> 'jsonb_object_keys'));

-- Compound index for user queries with skip_reasons
CREATE INDEX IF NOT EXISTS idx_webdav_sync_sessions_user_skip_reasons ON webdav_sync_sessions(user_id) 
    WHERE skip_reasons IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_webdav_directory_metrics_session_id ON webdav_directory_metrics(session_id);
CREATE INDEX IF NOT EXISTS idx_webdav_directory_metrics_user_id ON webdav_directory_metrics(user_id);
CREATE INDEX IF NOT EXISTS idx_webdav_directory_metrics_source_id ON webdav_directory_metrics(source_id);
CREATE INDEX IF NOT EXISTS idx_webdav_directory_metrics_path ON webdav_directory_metrics(directory_path);
CREATE INDEX IF NOT EXISTS idx_webdav_directory_metrics_started_at ON webdav_directory_metrics(started_at);

-- JSON indexes for error_types queries
CREATE INDEX IF NOT EXISTS idx_webdav_directory_metrics_error_types_gin ON webdav_directory_metrics USING gin(error_types);

-- Compound index for user queries with error_types
CREATE INDEX IF NOT EXISTS idx_webdav_directory_metrics_user_error_types ON webdav_directory_metrics(user_id) 
    WHERE error_types IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_webdav_request_metrics_session_id ON webdav_request_metrics(session_id);
CREATE INDEX IF NOT EXISTS idx_webdav_request_metrics_user_id ON webdav_request_metrics(user_id);
CREATE INDEX IF NOT EXISTS idx_webdav_request_metrics_source_id ON webdav_request_metrics(source_id);
CREATE INDEX IF NOT EXISTS idx_webdav_request_metrics_started_at ON webdav_request_metrics(started_at);
CREATE INDEX IF NOT EXISTS idx_webdav_request_metrics_operation_type ON webdav_request_metrics(operation_type);
CREATE INDEX IF NOT EXISTS idx_webdav_request_metrics_success ON webdav_request_metrics(success);

-- Trigger to automatically update session updated_at timestamp
CREATE OR REPLACE FUNCTION update_webdav_session_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS webdav_sync_sessions_updated_at ON webdav_sync_sessions;
CREATE TRIGGER webdav_sync_sessions_updated_at
    BEFORE UPDATE ON webdav_sync_sessions
    FOR EACH ROW
    EXECUTE FUNCTION update_webdav_session_updated_at();

-- Function to calculate session statistics on completion
CREATE OR REPLACE FUNCTION finalize_webdav_session_metrics(p_session_id UUID)
RETURNS VOID AS $$
DECLARE
    v_session webdav_sync_sessions%ROWTYPE;
    v_total_requests INTEGER;
    v_successful_requests INTEGER;
    v_failed_requests INTEGER;
    v_retry_attempts INTEGER;
    v_network_time_ms BIGINT;
    v_slowest_operation_ms BIGINT;
    v_slowest_operation_path TEXT;
BEGIN
    -- Get current session data
    SELECT * INTO v_session FROM webdav_sync_sessions WHERE id = p_session_id;
    
    IF NOT FOUND THEN
        RAISE NOTICE 'Session not found: %', p_session_id;
        RETURN;
    END IF;
    
    
    -- Calculate request statistics from webdav_request_metrics
    -- Use explicit casting to avoid any type issues
    SELECT 
        CAST(COUNT(*) AS INTEGER),
        CAST(COUNT(CASE WHEN success = true THEN 1 END) AS INTEGER),
        CAST(COUNT(CASE WHEN success = false THEN 1 END) AS INTEGER), 
        CAST(COUNT(CASE WHEN retry_attempt > 0 THEN 1 END) AS INTEGER),
        CAST(COALESCE(SUM(duration_ms), 0) AS BIGINT)
    INTO 
        v_total_requests,
        v_successful_requests,
        v_failed_requests,
        v_retry_attempts,
        v_network_time_ms
    FROM webdav_request_metrics 
    WHERE session_id = p_session_id;
    
    -- Get the slowest operation separately
    SELECT 
        duration_ms,
        target_path
    INTO 
        v_slowest_operation_ms,
        v_slowest_operation_path
    FROM webdav_request_metrics 
    WHERE session_id = p_session_id
    ORDER BY duration_ms DESC
    LIMIT 1;
    
    -- Update session with final metrics
    UPDATE webdav_sync_sessions SET
        completed_at = NOW(),
        duration_ms = EXTRACT(EPOCH FROM (NOW() - started_at)) * 1000,
        total_http_requests = COALESCE(v_total_requests, 0),
        successful_requests = COALESCE(v_successful_requests, 0),
        failed_requests = COALESCE(v_failed_requests, 0),
        retry_attempts = COALESCE(v_retry_attempts, 0),
        network_time_ms = COALESCE(v_network_time_ms, 0),
        slowest_operation_ms = v_slowest_operation_ms,
        slowest_operation_path = v_slowest_operation_path,
        processing_rate_files_per_sec = CASE 
            WHEN files_processed > 0 AND EXTRACT(EPOCH FROM (NOW() - started_at)) > 0 
            THEN files_processed / EXTRACT(EPOCH FROM (NOW() - started_at))
            ELSE 0 
        END,
        avg_file_size_bytes = CASE 
            WHEN files_processed > 0 
            THEN total_bytes_processed / files_processed 
            ELSE 0 
        END,
        status = CASE 
            WHEN status = 'in_progress' THEN 'completed'
            ELSE status 
        END,
        updated_at = NOW()
    WHERE id = p_session_id;
    
    RAISE NOTICE 'Session % finalized with % total requests, % successful', p_session_id, v_total_requests, v_successful_requests;
END;
$$ LANGUAGE plpgsql;

-- Function to get WebDAV metrics for a specific time period
CREATE OR REPLACE FUNCTION get_webdav_metrics_summary(
    p_user_id UUID DEFAULT NULL,
    p_source_id UUID DEFAULT NULL,
    p_start_time TIMESTAMPTZ DEFAULT NOW() - INTERVAL '24 hours',
    p_end_time TIMESTAMPTZ DEFAULT NOW()
)
RETURNS TABLE (
    total_sessions INTEGER,
    successful_sessions INTEGER,
    failed_sessions INTEGER,
    total_files_processed BIGINT,
    total_bytes_processed BIGINT,
    avg_session_duration_sec DOUBLE PRECISION,
    avg_processing_rate DOUBLE PRECISION,
    total_http_requests BIGINT,
    request_success_rate DOUBLE PRECISION,
    avg_request_duration_ms DOUBLE PRECISION,
    common_error_types JSONB
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        COUNT(*)::INTEGER as total_sessions,
        COUNT(*) FILTER (WHERE s.status = 'completed')::INTEGER as successful_sessions,
        COUNT(*) FILTER (WHERE s.status = 'failed')::INTEGER as failed_sessions,
        COALESCE(SUM(s.files_processed), 0)::BIGINT as total_files_processed,
        COALESCE(SUM(s.total_bytes_processed), 0)::BIGINT as total_bytes_processed,
        COALESCE(AVG(s.duration_ms / 1000.0), 0.0)::DOUBLE PRECISION as avg_session_duration_sec,
        COALESCE(AVG(s.processing_rate_files_per_sec), 0.0)::DOUBLE PRECISION as avg_processing_rate,
        COALESCE(SUM(s.total_http_requests), 0)::BIGINT as total_http_requests,
        CASE 
            WHEN SUM(s.total_http_requests) > 0 
            THEN (SUM(s.successful_requests)::DOUBLE PRECISION / SUM(s.total_http_requests) * 100.0)
            ELSE 0.0
        END::DOUBLE PRECISION as request_success_rate,
        COALESCE(
            (SELECT AVG(duration_ms)::DOUBLE PRECISION FROM webdav_request_metrics r 
             WHERE r.started_at BETWEEN p_start_time AND p_end_time
             AND (p_user_id IS NULL OR r.user_id = p_user_id)
             AND (p_source_id IS NULL OR r.source_id = p_source_id)),
            0.0
        )::DOUBLE PRECISION as avg_request_duration_ms,
        COALESCE(
            (SELECT jsonb_agg(jsonb_build_object('error_type', error_type, 'count', error_count))
             FROM (
                 SELECT error_type, COUNT(*) as error_count
                 FROM webdav_request_metrics r
                 WHERE r.started_at BETWEEN p_start_time AND p_end_time
                 AND r.success = false
                 AND r.error_type IS NOT NULL
                 AND (p_user_id IS NULL OR r.user_id = p_user_id)
                 AND (p_source_id IS NULL OR r.source_id = p_source_id)
                 GROUP BY error_type
                 ORDER BY error_count DESC
                 LIMIT 10
             ) error_summary),
            '[]'::jsonb
        ) as common_error_types
    FROM webdav_sync_sessions s
    WHERE s.started_at BETWEEN p_start_time AND p_end_time
    AND (p_user_id IS NULL OR s.user_id = p_user_id)
    AND (p_source_id IS NULL OR s.source_id = p_source_id);
END;
$$ LANGUAGE plpgsql;