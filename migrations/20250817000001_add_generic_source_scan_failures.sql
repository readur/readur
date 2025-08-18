-- Generic Source Scan Failures Tracking System
-- This migration creates a comprehensive failure tracking system for all source types (WebDAV, S3, Local Filesystem)

-- Create enum for generic source types
-- Use DO block to handle existing type gracefully
DO $$ BEGIN
    CREATE TYPE source_error_source_type AS ENUM (
    'webdav',       -- WebDAV/CalDAV servers
    's3',           -- S3-compatible object storage
    'local',        -- Local filesystem folders
    'dropbox',      -- Future: Dropbox integration
    'gdrive',       -- Future: Google Drive integration
    'onedrive'      -- Future: OneDrive integration
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Create enum for generic error types
DO $$ BEGIN
    CREATE TYPE source_error_type AS ENUM (
    'timeout',              -- Request or operation took too long
    'permission_denied',    -- Access denied or authentication failure
    'network_error',        -- Network connectivity issues
    'server_error',         -- Server returned error (404, 500, etc.)
    'path_too_long',        -- Path exceeds filesystem or protocol limits
    'invalid_characters',   -- Invalid characters in path/filename
    'too_many_items',       -- Directory has too many items
    'depth_limit',          -- Directory depth exceeds limit
    'size_limit',           -- File or directory size exceeds limit
    'xml_parse_error',      -- Malformed XML response (WebDAV specific)
    'json_parse_error',     -- Malformed JSON response (S3/API specific)
    'quota_exceeded',       -- Storage quota exceeded
    'rate_limited',         -- API rate limit exceeded
    'not_found',            -- Resource not found
    'conflict',             -- Conflict with existing resource
    'unsupported_operation', -- Operation not supported by source
    'unknown'               -- Unknown error type
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Create enum for error severity levels
DO $$ BEGIN
    CREATE TYPE source_error_severity AS ENUM (
    'low',      -- Can be retried, likely temporary (network issues)
    'medium',   -- May succeed with adjustments (timeouts, server errors)
    'high',     -- Unlikely to succeed without intervention (permissions, too many items)
    'critical'  -- Will never succeed, permanent issue (path too long, invalid characters)
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

-- Main table for tracking scan failures across all source types
CREATE TABLE IF NOT EXISTS source_scan_failures (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    source_type source_error_source_type NOT NULL,
    source_id UUID REFERENCES sources(id) ON DELETE CASCADE, -- Links to specific source configuration
    resource_path TEXT NOT NULL,  -- Path/key/identifier within the source
    
    -- Failure classification
    error_type source_error_type NOT NULL DEFAULT 'unknown',
    error_severity source_error_severity NOT NULL DEFAULT 'medium',
    failure_count INTEGER NOT NULL DEFAULT 1,
    consecutive_failures INTEGER NOT NULL DEFAULT 1,
    
    -- Timestamps
    first_failure_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_failure_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    last_retry_at TIMESTAMP WITH TIME ZONE,
    next_retry_at TIMESTAMP WITH TIME ZONE,
    
    -- Error details
    error_message TEXT,
    error_code TEXT,                    -- System/API specific error codes
    http_status_code INTEGER,           -- HTTP status codes where applicable
    
    -- Performance metrics
    response_time_ms INTEGER,           -- How long the request took
    response_size_bytes BIGINT,         -- Size of response (for timeout diagnosis)
    
    -- Resource characteristics
    resource_size_bytes BIGINT,         -- Size of the resource that failed
    resource_depth INTEGER,             -- Depth in hierarchy (for nested resources)
    estimated_item_count INTEGER,       -- Estimated number of items in directory
    
    -- Source-specific diagnostic data (flexible JSON field)
    diagnostic_data JSONB DEFAULT '{}',
    
    -- User actions
    user_excluded BOOLEAN DEFAULT FALSE,  -- User marked as permanently excluded
    user_notes TEXT,                      -- User-provided notes about the issue
    
    -- Retry strategy configuration
    retry_strategy TEXT DEFAULT 'exponential', -- Strategy: exponential, linear, fixed
    max_retries INTEGER DEFAULT 5,       -- Maximum number of retries
    retry_delay_seconds INTEGER DEFAULT 300, -- Base delay between retries
    
    -- Resolution tracking
    resolved BOOLEAN DEFAULT FALSE,
    resolved_at TIMESTAMP WITH TIME ZONE,
    resolution_method TEXT,              -- How it was resolved (automatic, manual, etc.)
    resolution_notes TEXT,               -- Additional notes about resolution
    
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    
    -- Unique constraint to prevent duplicates per source
    CONSTRAINT unique_source_resource_failure UNIQUE (user_id, source_type, source_id, resource_path)
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_source_scan_failures_user_id ON source_scan_failures(user_id);
CREATE INDEX IF NOT EXISTS idx_source_scan_failures_source_type ON source_scan_failures(source_type);
CREATE INDEX IF NOT EXISTS idx_source_scan_failures_source_id ON source_scan_failures(source_id);
CREATE INDEX IF NOT EXISTS idx_source_scan_failures_error_type ON source_scan_failures(error_type);
CREATE INDEX IF NOT EXISTS idx_source_scan_failures_error_severity ON source_scan_failures(error_severity);
CREATE INDEX IF NOT EXISTS idx_source_scan_failures_resolved ON source_scan_failures(resolved);
CREATE INDEX IF NOT EXISTS idx_source_scan_failures_next_retry ON source_scan_failures(next_retry_at) WHERE NOT resolved AND NOT user_excluded;
CREATE INDEX IF NOT EXISTS idx_source_scan_failures_resource_path ON source_scan_failures(resource_path);
CREATE INDEX IF NOT EXISTS idx_source_scan_failures_composite_active ON source_scan_failures(user_id, source_type, resolved, user_excluded) WHERE NOT resolved;

-- GIN index for flexible JSON diagnostic data queries
CREATE INDEX IF NOT EXISTS idx_source_scan_failures_diagnostic_data ON source_scan_failures USING GIN (diagnostic_data);

-- Function to calculate next retry time with configurable backoff strategies
CREATE OR REPLACE FUNCTION calculate_source_retry_time(
    failure_count INTEGER,
    retry_strategy TEXT,
    base_delay_seconds INTEGER,
    max_delay_seconds INTEGER DEFAULT 86400  -- 24 hours max
) RETURNS TIMESTAMP WITH TIME ZONE AS $$
DECLARE
    delay_seconds INTEGER;
BEGIN
    CASE retry_strategy
        WHEN 'exponential' THEN
            -- Exponential backoff: delay = base * 2^(failure_count - 1)
            delay_seconds := LEAST(
                base_delay_seconds * POWER(2, LEAST(failure_count - 1, 10)),
                max_delay_seconds
            );
        WHEN 'linear' THEN
            -- Linear backoff: delay = base * failure_count
            delay_seconds := LEAST(
                base_delay_seconds * failure_count,
                max_delay_seconds
            );
        WHEN 'fixed' THEN
            -- Fixed delay
            delay_seconds := base_delay_seconds;
        ELSE
            -- Default to exponential
            delay_seconds := LEAST(
                base_delay_seconds * POWER(2, LEAST(failure_count - 1, 10)),
                max_delay_seconds
            );
    END CASE;
    
    RETURN NOW() + (delay_seconds || ' seconds')::INTERVAL;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Function to automatically determine error severity based on error type and context
CREATE OR REPLACE FUNCTION classify_error_severity(
    p_error_type source_error_type,
    p_http_status_code INTEGER DEFAULT NULL,
    p_failure_count INTEGER DEFAULT 1,
    p_error_message TEXT DEFAULT NULL
) RETURNS source_error_severity AS $$
BEGIN
    CASE p_error_type
        -- Critical errors that won't resolve automatically
        WHEN 'path_too_long', 'invalid_characters' THEN
            RETURN 'critical'::source_error_severity;
            
        -- High severity errors requiring intervention
        WHEN 'permission_denied', 'quota_exceeded', 'too_many_items', 'depth_limit', 'size_limit' THEN
            RETURN 'high'::source_error_severity;
            
        -- Context-dependent severity
        WHEN 'server_error' THEN
            IF p_http_status_code = 404 THEN
                RETURN 'critical'::source_error_severity;  -- Resource doesn't exist
            ELSIF p_http_status_code >= 500 THEN
                RETURN 'medium'::source_error_severity;    -- Server issues, may recover
            ELSE
                RETURN 'medium'::source_error_severity;
            END IF;
            
        WHEN 'not_found' THEN
            RETURN 'critical'::source_error_severity;
            
        WHEN 'timeout' THEN
            -- Repeated timeouts indicate systemic issues
            IF p_failure_count > 5 THEN
                RETURN 'high'::source_error_severity;
            ELSE
                RETURN 'medium'::source_error_severity;
            END IF;
            
        -- Low severity, likely temporary
        WHEN 'network_error', 'rate_limited' THEN
            RETURN 'low'::source_error_severity;
            
        -- Medium severity by default
        ELSE
            RETURN 'medium'::source_error_severity;
    END CASE;
END;
$$ LANGUAGE plpgsql IMMUTABLE;

-- Function to record or update a source scan failure
CREATE OR REPLACE FUNCTION record_source_scan_failure(
    p_user_id UUID,
    p_source_type source_error_source_type,
    p_source_id UUID,
    p_resource_path TEXT,
    p_error_type source_error_type,
    p_error_message TEXT,
    p_error_code TEXT DEFAULT NULL,
    p_http_status_code INTEGER DEFAULT NULL,
    p_response_time_ms INTEGER DEFAULT NULL,
    p_response_size_bytes BIGINT DEFAULT NULL,
    p_resource_size_bytes BIGINT DEFAULT NULL,
    p_diagnostic_data JSONB DEFAULT NULL
) RETURNS UUID AS $$
DECLARE
    v_failure_id UUID;
    v_existing_count INTEGER DEFAULT 0;
    v_severity source_error_severity;
    v_retry_strategy TEXT DEFAULT 'exponential';
    v_base_delay INTEGER DEFAULT 300;
BEGIN
    -- Determine severity based on error type and context
    v_severity := classify_error_severity(p_error_type, p_http_status_code, 1, p_error_message);
    
    -- Adjust retry strategy based on error type
    CASE p_error_type
        WHEN 'rate_limited' THEN
            v_retry_strategy := 'linear';
            v_base_delay := 600; -- 10 minutes for rate limiting
        WHEN 'network_error' THEN
            v_retry_strategy := 'exponential';
            v_base_delay := 60; -- 1 minute for network issues
        WHEN 'timeout' THEN
            v_retry_strategy := 'exponential';
            v_base_delay := 900; -- 15 minutes for timeouts
        ELSE
            v_retry_strategy := 'exponential';
            v_base_delay := 300; -- 5 minutes default
    END CASE;
    
    -- Insert or update the failure record
    INSERT INTO source_scan_failures (
        user_id,
        source_type,
        source_id,
        resource_path,
        error_type,
        error_severity,
        failure_count,
        consecutive_failures,
        error_message,
        error_code,
        http_status_code,
        response_time_ms,
        response_size_bytes,
        resource_size_bytes,
        resource_depth,
        estimated_item_count,
        diagnostic_data,
        retry_strategy,
        retry_delay_seconds,
        next_retry_at
    ) VALUES (
        p_user_id,
        p_source_type,
        p_source_id,
        p_resource_path,
        p_error_type,
        v_severity,
        1,
        1,
        p_error_message,
        p_error_code,
        p_http_status_code,
        p_response_time_ms,
        p_response_size_bytes,
        p_resource_size_bytes,
        array_length(string_to_array(p_resource_path, '/'), 1) - 1,
        NULL, -- Will be filled in by source-specific logic
        COALESCE(p_diagnostic_data, '{}'::jsonb),
        v_retry_strategy,
        v_base_delay,
        calculate_source_retry_time(1, v_retry_strategy, v_base_delay, 86400)
    )
    ON CONFLICT (user_id, source_type, source_id, resource_path) DO UPDATE SET
        error_type = EXCLUDED.error_type,
        error_severity = classify_error_severity(EXCLUDED.error_type, EXCLUDED.http_status_code, source_scan_failures.failure_count + 1, EXCLUDED.error_message),
        failure_count = source_scan_failures.failure_count + 1,
        consecutive_failures = source_scan_failures.consecutive_failures + 1,
        last_failure_at = NOW(),
        error_message = EXCLUDED.error_message,
        error_code = EXCLUDED.error_code,
        http_status_code = EXCLUDED.http_status_code,
        response_time_ms = EXCLUDED.response_time_ms,
        response_size_bytes = EXCLUDED.response_size_bytes,
        resource_size_bytes = EXCLUDED.resource_size_bytes,
        diagnostic_data = COALESCE(EXCLUDED.diagnostic_data, source_scan_failures.diagnostic_data),
        next_retry_at = calculate_source_retry_time(
            source_scan_failures.failure_count + 1,
            source_scan_failures.retry_strategy,
            source_scan_failures.retry_delay_seconds,
            86400
        ),
        resolved = FALSE,
        updated_at = NOW()
    RETURNING id INTO v_failure_id;
    
    RETURN v_failure_id;
END;
$$ LANGUAGE plpgsql;

-- Function to reset a failure for retry
CREATE OR REPLACE FUNCTION reset_source_scan_failure(
    p_user_id UUID,
    p_source_type source_error_source_type,
    p_source_id UUID,
    p_resource_path TEXT
) RETURNS BOOLEAN AS $$
DECLARE
    v_updated INTEGER;
BEGIN
    UPDATE source_scan_failures
    SET 
        consecutive_failures = 0,
        last_retry_at = NOW(),
        next_retry_at = NOW(),  -- Retry immediately
        resolved = FALSE,
        user_excluded = FALSE,
        updated_at = NOW()
    WHERE user_id = p_user_id 
        AND source_type = p_source_type
        AND (source_id = p_source_id OR (source_id IS NULL AND p_source_id IS NULL))
        AND resource_path = p_resource_path
        AND NOT resolved;
    
    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN v_updated > 0;
END;
$$ LANGUAGE plpgsql;

-- Function to mark a failure as resolved
CREATE OR REPLACE FUNCTION resolve_source_scan_failure(
    p_user_id UUID,
    p_source_type source_error_source_type,
    p_source_id UUID,
    p_resource_path TEXT,
    p_resolution_method TEXT DEFAULT 'automatic'
) RETURNS BOOLEAN AS $$
DECLARE
    v_updated INTEGER;
BEGIN
    UPDATE source_scan_failures
    SET 
        resolved = TRUE,
        resolved_at = NOW(),
        resolution_method = p_resolution_method,
        consecutive_failures = 0,
        updated_at = NOW()
    WHERE user_id = p_user_id 
        AND source_type = p_source_type
        AND (source_id = p_source_id OR (source_id IS NULL AND p_source_id IS NULL))
        AND resource_path = p_resource_path
        AND NOT resolved;
    
    GET DIAGNOSTICS v_updated = ROW_COUNT;
    RETURN v_updated > 0;
END;
$$ LANGUAGE plpgsql;

-- View for active failures that need attention across all source types
CREATE OR REPLACE VIEW active_source_scan_failures AS
SELECT 
    ssf.*,
    u.username,
    u.email,
    s.name as source_name,
    s.source_type as configured_source_type,
    CASE 
        WHEN ssf.failure_count > 20 THEN 'chronic'
        WHEN ssf.failure_count > 10 THEN 'persistent'
        WHEN ssf.failure_count > 3 THEN 'recurring'
        ELSE 'recent'
    END as failure_status,
    CASE
        WHEN ssf.next_retry_at < NOW() AND NOT ssf.user_excluded AND NOT ssf.resolved THEN 'ready_for_retry'
        WHEN ssf.user_excluded THEN 'excluded'
        WHEN ssf.error_severity = 'critical' THEN 'needs_intervention'
        WHEN ssf.resolved THEN 'resolved'
        ELSE 'scheduled'
    END as action_status
FROM source_scan_failures ssf
JOIN users u ON ssf.user_id = u.id
LEFT JOIN sources s ON ssf.source_id = s.id
WHERE NOT ssf.resolved;

-- Trigger to update the updated_at timestamp
CREATE OR REPLACE FUNCTION update_source_scan_failures_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS update_source_scan_failures_updated_at ON source_scan_failures;
CREATE TRIGGER update_source_scan_failures_updated_at
    BEFORE UPDATE ON source_scan_failures
    FOR EACH ROW
    EXECUTE FUNCTION update_source_scan_failures_updated_at();

-- Comments for documentation
COMMENT ON TABLE source_scan_failures IS 'Generic failure tracking for all source types (WebDAV, S3, Local, etc.) with detailed diagnostics and configurable retry strategies';
COMMENT ON COLUMN source_scan_failures.source_type IS 'Type of source (webdav, s3, local, etc.)';
COMMENT ON COLUMN source_scan_failures.source_id IS 'Reference to the specific source configuration (nullable for backward compatibility)';
COMMENT ON COLUMN source_scan_failures.resource_path IS 'Path/key/identifier of the resource that failed (directory, file, or object key)';
COMMENT ON COLUMN source_scan_failures.error_type IS 'Categorized type of error for analysis and handling across all source types';
COMMENT ON COLUMN source_scan_failures.error_severity IS 'Severity level determining retry strategy and user notification priority';
COMMENT ON COLUMN source_scan_failures.diagnostic_data IS 'Flexible JSONB field for storing source-specific diagnostic information';
COMMENT ON COLUMN source_scan_failures.retry_strategy IS 'Retry strategy: exponential, linear, or fixed delay';
COMMENT ON COLUMN source_scan_failures.user_excluded IS 'User has marked this resource to be permanently excluded from scanning';
COMMENT ON COLUMN source_scan_failures.consecutive_failures IS 'Number of consecutive failures without a successful scan';