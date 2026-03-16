-- Add progress tracking fields to ocr_queue for page-level OCR progress
ALTER TABLE ocr_queue ADD COLUMN IF NOT EXISTS progress_current INTEGER DEFAULT 0;
ALTER TABLE ocr_queue ADD COLUMN IF NOT EXISTS progress_total INTEGER DEFAULT 0;
