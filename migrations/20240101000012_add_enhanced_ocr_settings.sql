-- Add enhanced OCR processing settings with conservative defaults
ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_brightness_boost REAL DEFAULT 1.0;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_contrast_multiplier REAL DEFAULT 1.2;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_noise_reduction_level INTEGER DEFAULT 1;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_sharpening_strength REAL DEFAULT 0.5;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_morphological_operations BOOLEAN DEFAULT false;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_adaptive_threshold_window_size INTEGER DEFAULT 15;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_histogram_equalization BOOLEAN DEFAULT false;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_upscale_factor REAL DEFAULT 1.0;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_max_image_width INTEGER DEFAULT 3000;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_max_image_height INTEGER DEFAULT 3000;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS save_processed_images BOOLEAN DEFAULT false;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_quality_threshold_brightness REAL DEFAULT 0.3;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_quality_threshold_contrast REAL DEFAULT 0.2;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_quality_threshold_noise REAL DEFAULT 0.7;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_quality_threshold_sharpness REAL DEFAULT 0.3;
ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_skip_enhancement BOOLEAN DEFAULT false;

-- Create processed_images table for storing preprocessed images
CREATE TABLE IF NOT EXISTS processed_images (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    original_image_path TEXT NOT NULL,
    processed_image_path TEXT NOT NULL,
    processing_parameters JSONB NOT NULL DEFAULT '{}',
    processing_steps TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
    image_width INTEGER NOT NULL,
    image_height INTEGER NOT NULL,
    file_size BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create indexes for the processed_images table
CREATE INDEX IF NOT EXISTS idx_processed_images_document_id ON processed_images(document_id);
CREATE INDEX IF NOT EXISTS idx_processed_images_user_id ON processed_images(user_id);
CREATE INDEX IF NOT EXISTS idx_processed_images_created_at ON processed_images(created_at);

-- Update existing settings with conservative default values for new OCR settings
UPDATE settings SET 
    ocr_brightness_boost = 1.0,
    ocr_contrast_multiplier = 1.2,
    ocr_noise_reduction_level = 1,
    ocr_sharpening_strength = 0.5,
    ocr_morphological_operations = false,
    ocr_adaptive_threshold_window_size = 15,
    ocr_histogram_equalization = false,
    ocr_upscale_factor = 1.0,
    ocr_max_image_width = 3000,
    ocr_max_image_height = 3000,
    save_processed_images = false,
    ocr_quality_threshold_brightness = 0.3,
    ocr_quality_threshold_contrast = 0.2,
    ocr_quality_threshold_noise = 0.7,
    ocr_quality_threshold_sharpness = 0.3,
    ocr_skip_enhancement = false
WHERE ocr_brightness_boost IS NULL;