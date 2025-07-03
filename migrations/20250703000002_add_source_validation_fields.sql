-- Add validation status fields to sources table
ALTER TABLE sources 
ADD COLUMN validation_status TEXT DEFAULT NULL,
ADD COLUMN last_validation_at TIMESTAMP WITH TIME ZONE DEFAULT NULL,
ADD COLUMN validation_score INTEGER DEFAULT NULL CHECK (validation_score >= 0 AND validation_score <= 100),
ADD COLUMN validation_issues TEXT DEFAULT NULL;

-- Create index for querying validation status
CREATE INDEX idx_sources_validation_status ON sources (validation_status);
CREATE INDEX idx_sources_last_validation_at ON sources (last_validation_at);

-- Add comments for documentation
COMMENT ON COLUMN sources.validation_status IS 'Current validation status: "healthy", "warning", "critical", "validating", or NULL';
COMMENT ON COLUMN sources.last_validation_at IS 'Timestamp of the last validation check';
COMMENT ON COLUMN sources.validation_score IS 'Health score from 0-100, where 100 is perfect health';
COMMENT ON COLUMN sources.validation_issues IS 'JSON array of validation issues and recommendations';