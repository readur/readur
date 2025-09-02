-- Add office document extraction settings to the settings table
-- This migration adds timeout controls for Office document extraction using XML parsing

-- Add office extraction timeout column (default: 120 seconds)
ALTER TABLE settings 
ADD COLUMN IF NOT EXISTS office_extraction_timeout_seconds INTEGER NOT NULL DEFAULT 120
CHECK (office_extraction_timeout_seconds > 0 AND office_extraction_timeout_seconds <= 600);

-- Add office extraction detailed logging column (default: false for production)
ALTER TABLE settings 
ADD COLUMN IF NOT EXISTS office_extraction_enable_detailed_logging BOOLEAN NOT NULL DEFAULT false;

-- Add comment to document the new columns
COMMENT ON COLUMN settings.office_extraction_timeout_seconds IS 
'Timeout in seconds for office document extraction (1-600 seconds, default: 120)';

COMMENT ON COLUMN settings.office_extraction_enable_detailed_logging IS 
'Enable detailed logging for office document extraction operations (default: false)';

-- The default values are already set in the column definitions above
-- No need to insert default settings as they should be created when users are created