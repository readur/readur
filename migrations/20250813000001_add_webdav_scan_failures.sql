-- WebDAV Scan Failures Tracking System
-- This migration creates a comprehensive failure tracking system for WebDAV directory scans

-- Create enum for failure types
CREATE TYPE webdav_scan_failure_type AS ENUM (
    'timeout',           -- Directory scan took too long
    'path_too_long',     -- Path exceeds filesystem limits
    'permission_denied', -- Access denied
    'invalid_characters',-- Invalid characters in path
    'network_error',     -- Network connectivity issues
    'server_error',      -- Server returned error (404, 500, etc.)
    'xml_parse_error',   -- Malformed XML response
    'too_many_items',    -- Directory has too many items
    'depth_limit',       -- Directory depth exceeds limit
    'size_limit',        -- Directory size exceeds limit
    'unknown'            -- Unknown error type
);

-- Create enum for failure severity
CREATE TYPE webdav_scan_failure_severity AS ENUM (
    'low',      -- Can be retried, likely temporary
    'medium',   -- May succeed with adjustments
    'high',     -- Unlikely to succeed without intervention
    'critical'  -- Will never succeed, permanent issue
);

-- Main table for tracking scan failures
CREATE TABLE IF NOT EXISTS webdav_scan_failures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    directory_path TEXT NOT NULL,
    
    -- Failure tracking
    failure_type webdav_scan_failure_type NOT NULL DEFAULT 'unknown',
    failure_severity webdav_scan_failure_severity NOT NULL DEFAULT 'medium',
    failure_count INTEGER NOT NULL DEFAULT 1,
    consecutive_failures INTEGER NOT NULL DEFAULT 1,
    
    -- Timestamps
    first_failure_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_failure_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_retry_at TIMESTAMP WITH TIME ZONE,
    next_retry_at TIMESTAMP WITH TIME ZONE,
    
    -- Error details
    error_message TEXT,
    error_code TEXT,
    http_status_code INTEGER,
    
    -- Diagnostic information
    response_time_ms INTEGER,        -- How long the request took
    response_size_bytes BIGINT,      -- Size of response (for timeout diagnosis)
    path_length INTEGER,              -- Length of the path
    directory_depth INTEGER,          -- How deep in the hierarchy
    estimated_item_count INTEGER,     -- Estimated number of items
    server_type TEXT,                 -- WebDAV server type
    server_version TEXT,              -- Server version if available
    
    -- Additional context
    diagnostic_data JSONB,            -- Flexible field for additional diagnostics
    
    -- User actions
    user_excluded BOOLEAN DEFAULT FALSE,  -- User marked as permanently excluded
    user_notes TEXT,                      -- User-provided notes about the issue
    
    -- Retry strategy
    retry_strategy TEXT,              -- Strategy for retrying (exponential, linear, etc.)
    max_retries INTEGER DEFAULT 5,   -- Maximum number of retries
    retry_delay_seconds INTEGER DEFAULT 300, -- Base delay between retries
    
    -- Resolution tracking
    resolved BOOLEAN DEFAULT FALSE,
    resolved_at TIMESTAMP WITH TIME ZONE,
    resolution_method TEXT,           -- How it was resolved
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Unique constraint to prevent duplicates
    CONSTRAINT unique_user_directory_failure UNIQUE (user_id, directory_path)
);

-- Create indexes for efficient querying
CREATE INDEX idx_webdav_scan_failures_user_id ON webdav_scan_failures(user_id);
CREATE INDEX idx_webdav_scan_failures_severity ON webdav_scan_failures(failure_severity);
CREATE INDEX idx_webdav_scan_failures_type ON webdav_scan_failures(failure_type);
CREATE INDEX idx_webdav_scan_failures_resolved ON webdav_scan_failures(resolved);
CREATE INDEX idx_webdav_scan_failures_next_retry ON webdav_scan_failures(next_retry_at) WHERE NOT resolved AND NOT user_excluded;
CREATE INDEX idx_webdav_scan_failures_path ON webdav_scan_failures(directory_path);

-- Function to calculate next retry time with exponential backoff
CREATE OR REPLACE FUNCTION calculate_next_retry_time(
    failure_count INTEGER,
    base_delay_seconds INTEGER,
    max_delay_seconds INTEGER DEFAULT 86400  -- 24 hours max
) RETURNS TIMESTAMP WITH TIME ZONE AS $$
DECLARE
    delay_seconds INTEGER;
BEGIN
    -- Exponential backoff: delay = base * 2^(failure_count - 1)
    -- Cap at max_delay_seconds
    delay_seconds := LEAST(
        base_delay_seconds * POWER(2, LEAST(failure_count - 1, 10)),
        max_delay_seconds
    );
    
    RETURN NOW() + (delay_seconds || ' seconds')::INTERVAL;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Function to record or update a scan failure
CREATE OR REPLACE FUNCTION record_webdav_scan_failure(
    p_user_id UUID,
    p_directory_path TEXT,
    p_failure_type webdav_scan_failure_type,
    p_error_message TEXT,
    p_error_code TEXT DEFAULT NULL,
    p_http_status_code INTEGER DEFAULT NULL,
    p_response_time_ms INTEGER DEFAULT NULL,
    p_response_size_bytes BIGINT DEFAULT NULL,
    p_diagnostic_data JSONB DEFAULT NULL
) RETURNS UUID AS $$
DECLARE
    v_failure_id UUID;
    v_existing_count INTEGER;
    v_severity webdav_scan_failure_severity;
BEGIN
    -- Determine severity based on failure type
    v_severity := CASE p_failure_type
        WHEN 'timeout' THEN 'medium'::webdav_scan_failure_severity
        WHEN 'path_too_long' THEN 'critical'::webdav_scan_failure_severity
        WHEN 'permission_denied' THEN 'high'::webdav_scan_failure_severity
        WHEN 'invalid_characters' THEN 'critical'::webdav_scan_failure_severity
        WHEN 'network_error' THEN 'low'::webdav_scan_failure_severity
        WHEN 'server_error' THEN 
            CASE 
                WHEN p_http_status_code = 404 THEN 'critical'::webdav_scan_failure_severity
                WHEN p_http_status_code >= 500 THEN 'medium'::webdav_scan_failure_severity
                ELSE 'medium'::webdav_scan_failure_severity
            END
        WHEN 'xml_parse_error' THEN 'high'::webdav_scan_failure_severity
        WHEN 'too_many_items' THEN 'high'::webdav_scan_failure_severity
        WHEN 'depth_limit' THEN 'high'::webdav_scan_failure_severity
        WHEN 'size_limit' THEN 'high'::webdav_scan_failure_severity
        ELSE 'medium'::webdav_scan_failure_severity
    END;
    
    -- Insert or update the failure record
    INSERT INTO webdav_scan_failures (
        user_id,
        directory_path,
        failure_type,
        failure_severity,
        failure_count,
        consecutive_failures,
        error_message,
        error_code,
        http_status_code,
        response_time_ms,
        response_size_bytes,
        path_length,
        directory_depth,
        diagnostic_data,
        next_retry_at
    ) VALUES (
        p_user_id,
        p_directory_path,
        p_failure_type,
        v_severity,
        1,
        1,
        p_error_message,
        p_error_code,
        p_http_status_code,
        p_response_time_ms,
        p_response_size_bytes,
        LENGTH(p_directory_path),
        array_length(string_to_array(p_directory_path, '/'), 1) - 1,
        p_diagnostic_data,
        calculate_next_retry_time(1, 300, 86400)
    )
    ON CONFLICT (user_id, directory_path) DO UPDATE SET
        failure_type = EXCLUDED.failure_type,
        failure_severity = EXCLUDED.failure_severity,
        failure_count = webdav_scan_failures.failure_count + 1,
        consecutive_failures = webdav_scan_failures.consecutive_failures + 1,
        last_failure_at = NOW(),
        error_message = EXCLUDED.error_message,
        error_code = EXCLUDED.error_code,
        http_status_code = EXCLUDED.http_status_code,
        response_time_ms = EXCLUDED.response_time_ms,
        response_size_bytes = EXCLUDED.response_size_bytes,
        diagnostic_data = COALESCE(EXCLUDED.diagnostic_data, webdav_scan_failures.diagnostic_data),
        next_retry_at = calculate_next_retry_time(
            webdav_scan_failures.failure_count + 1,
            webdav_scan_failures.retry_delay_seconds,
            86400
        ),
        resolved = FALSE,
        updated_at = NOW()
    RETURNING id INTO v_failure_id;
    
    RETURN v_failure_id;
END;
$$ LANGUAGE plpgsql;

-- Function to reset a failure for retry
CREATE OR REPLACE FUNCTION reset_webdav_scan_failure(
    p_user_id UUID,
    p_directory_path TEXT
) RETURNS BOOLEAN AS $$
DECLARE
    v_updated INTEGER;
BEGIN
    UPDATE webdav_scan_failures
    SET 
        consecutive_failures = 0,
        last_retry_at = NOW(),
        next_retry_at = NOW(),  -- Retry immediately
        resolved = FALSE,
        user_excluded = FALSE,
        updated_at = NOW()
    WHERE user_id = p_user_id 
        AND directory_path = p_directory_path
        AND NOT resolved;
    
    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN v_updated > 0;
END;
$$ LANGUAGE plpgsql;

-- Function to mark a failure as resolved
CREATE OR REPLACE FUNCTION resolve_webdav_scan_failure(
    p_user_id UUID,
    p_directory_path TEXT,
    p_resolution_method TEXT DEFAULT 'automatic'
) RETURNS BOOLEAN AS $$
DECLARE
    v_updated INTEGER;
BEGIN
    UPDATE webdav_scan_failures
    SET 
        resolved = TRUE,
        resolved_at = NOW(),
        resolution_method = p_resolution_method,
        consecutive_failures = 0,
        updated_at = NOW()
    WHERE user_id = p_user_id 
        AND directory_path = p_directory_path
        AND NOT resolved;
    
    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN v_updated > 0;
END;
$$ LANGUAGE plpgsql;

-- View for active failures that need attention
CREATE VIEW active_webdav_scan_failures AS
SELECT 
    wsf.*,
    u.username,
    u.email,
    CASE 
        WHEN wsf.failure_count > 10 THEN 'chronic'
        WHEN wsf.failure_count > 5 THEN 'persistent'
        WHEN wsf.failure_count > 2 THEN 'recurring'
        ELSE 'recent'
    END as failure_status,
    CASE
        WHEN wsf.next_retry_at < NOW() THEN 'ready_for_retry'
        WHEN wsf.user_excluded THEN 'excluded'
        WHEN wsf.failure_severity = 'critical' THEN 'needs_intervention'
        ELSE 'scheduled'
    END as action_status
FROM webdav_scan_failures wsf
JOIN users u ON wsf.user_id = u.id
WHERE NOT wsf.resolved;

-- Trigger to update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_webdav_scan_failures_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_webdav_scan_failures_updated_at
    BEFORE UPDATE ON webdav_scan_failures
    FOR EACH ROW
    EXECUTE FUNCTION update_webdav_scan_failures_updated_at();

-- Comments for documentation
COMMENT ON TABLE webdav_scan_failures IS 'Tracks failures during WebDAV directory scanning with detailed diagnostics';
COMMENT ON COLUMN webdav_scan_failures.failure_type IS 'Categorized type of failure for analysis and handling';
COMMENT ON COLUMN webdav_scan_failures.failure_severity IS 'Severity level determining retry strategy and user notification';
COMMENT ON COLUMN webdav_scan_failures.diagnostic_data IS 'Flexible JSON field for storing additional diagnostic information';
COMMENT ON COLUMN webdav_scan_failures.user_excluded IS 'User has marked this directory to be permanently excluded from scanning';
COMMENT ON COLUMN webdav_scan_failures.consecutive_failures IS 'Number of consecutive failures without a successful scan';