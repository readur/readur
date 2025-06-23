-- Add missing OCR columns to documents table for existing databases
ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_error TEXT;
ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_completed_at TIMESTAMPTZ;