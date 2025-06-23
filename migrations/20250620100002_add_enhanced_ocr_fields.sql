ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_confidence REAL;

ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_word_count INT;

ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_processing_time_ms INT;

ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_page_segmentation_mode INT DEFAULT 3;

ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_engine_mode INT DEFAULT 3;

ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_min_confidence REAL DEFAULT 30.0;

ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_dpi INT DEFAULT 300;

ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_enhance_contrast BOOLEAN DEFAULT true;

ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_remove_noise BOOLEAN DEFAULT true;

ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_detect_orientation BOOLEAN DEFAULT true;

ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_whitelist_chars TEXT;

ALTER TABLE settings ADD COLUMN IF NOT EXISTS ocr_blacklist_chars TEXT;

CREATE INDEX IF NOT EXISTS idx_documents_ocr_confidence ON documents(ocr_confidence) WHERE ocr_confidence IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_documents_ocr_word_count ON documents(ocr_word_count) WHERE ocr_word_count IS NOT NULL;