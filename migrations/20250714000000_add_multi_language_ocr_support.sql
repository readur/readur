-- Migration: Add multi-language OCR support
-- This migration adds support for multiple OCR languages per user

-- Add new columns for multi-language support (if they don't exist)
ALTER TABLE settings 
ADD COLUMN IF NOT EXISTS preferred_languages JSONB DEFAULT '["eng"]'::jsonb,
ADD COLUMN IF NOT EXISTS primary_language VARCHAR(10) DEFAULT 'eng',
ADD COLUMN IF NOT EXISTS auto_detect_language_combination BOOLEAN DEFAULT false;

-- Migrate existing ocr_language data to new preferred_languages array (only if not already migrated)
UPDATE settings 
SET preferred_languages = jsonb_build_array(COALESCE(ocr_language, 'eng')),
    primary_language = COALESCE(ocr_language, 'eng')
WHERE preferred_languages = '["eng"]'::jsonb AND ocr_language IS NOT NULL AND ocr_language != 'eng';

-- Create index for efficient querying of preferred languages
CREATE INDEX IF NOT EXISTS idx_settings_preferred_languages ON settings USING gin(preferred_languages);
CREATE INDEX IF NOT EXISTS idx_settings_primary_language ON settings(primary_language);

-- Add constraints (if they don't exist)
DO $$
BEGIN
    -- Add constraint to ensure primary_language is always in preferred_languages
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'check_primary_language_in_preferred') THEN
        ALTER TABLE settings 
        ADD CONSTRAINT check_primary_language_in_preferred 
        CHECK (preferred_languages ? primary_language);
    END IF;

    -- Add constraint to limit number of preferred languages (max 4 for performance)
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'check_max_preferred_languages') THEN
        ALTER TABLE settings 
        ADD CONSTRAINT check_max_preferred_languages 
        CHECK (jsonb_array_length(preferred_languages) <= 4);
    END IF;

    -- Add constraint to ensure valid primary language code (3-letter ISO codes)
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'check_valid_primary_language_code') THEN
        ALTER TABLE settings 
        ADD CONSTRAINT check_valid_primary_language_code 
        CHECK (primary_language ~ '^[a-z]{3}(_[A-Z]{2})?$');
    END IF;
END
$$;

-- Note: preferred_languages validation is handled in application code due to PostgreSQL subquery limitations in CHECK constraints

-- Update existing users who don't have settings yet
INSERT INTO settings (user_id, preferred_languages, primary_language, auto_detect_language_combination)
SELECT 
    u.id,
    '["eng"]'::jsonb,
    'eng',
    false
FROM users u
WHERE NOT EXISTS (
    SELECT 1 FROM settings s WHERE s.user_id = u.id
);

-- Add comments for documentation
COMMENT ON COLUMN settings.preferred_languages IS 'Array of 3-letter ISO language codes for OCR processing, max 4 languages';
COMMENT ON COLUMN settings.primary_language IS 'Primary language code that should be listed first in OCR processing';
COMMENT ON COLUMN settings.auto_detect_language_combination IS 'Whether to automatically suggest language combinations based on document content';