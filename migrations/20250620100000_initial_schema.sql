-- Create extensions
CREATE EXTENSION IF NOT EXISTS "pg_trgm";

-- Create users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username VARCHAR(255) UNIQUE NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Create documents table
CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    filename VARCHAR(255) NOT NULL,
    original_filename VARCHAR(255) NOT NULL,
    file_path VARCHAR(500) NOT NULL,
    file_size BIGINT NOT NULL,
    mime_type VARCHAR(100) NOT NULL,
    content TEXT,
    ocr_text TEXT,
    ocr_confidence REAL,
    ocr_word_count INT,
    ocr_processing_time_ms INT,
    ocr_status VARCHAR(20) DEFAULT 'pending',
    ocr_error TEXT,
    ocr_completed_at TIMESTAMPTZ,
    tags TEXT[] DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_documents_user_id ON documents(user_id);
CREATE INDEX IF NOT EXISTS idx_documents_filename ON documents(filename);
CREATE INDEX IF NOT EXISTS idx_documents_mime_type ON documents(mime_type);
CREATE INDEX IF NOT EXISTS idx_documents_tags ON documents USING GIN(tags);
CREATE INDEX IF NOT EXISTS idx_documents_content_search ON documents USING GIN(to_tsvector('english', COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')));
CREATE INDEX IF NOT EXISTS idx_documents_filename_trgm ON documents USING GIN(filename gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_documents_content_trgm ON documents USING GIN((COALESCE(content, '') || ' ' || COALESCE(ocr_text, '')) gin_trgm_ops);
CREATE INDEX IF NOT EXISTS idx_documents_ocr_confidence ON documents(ocr_confidence) WHERE ocr_confidence IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_documents_ocr_word_count ON documents(ocr_word_count) WHERE ocr_word_count IS NOT NULL;

-- Create settings table
CREATE TABLE IF NOT EXISTS settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE UNIQUE,
    ocr_language VARCHAR(10) DEFAULT 'eng',
    concurrent_ocr_jobs INT DEFAULT 4,
    ocr_timeout_seconds INT DEFAULT 300,
    max_file_size_mb INT DEFAULT 50,
    allowed_file_types TEXT[] DEFAULT ARRAY['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'],
    auto_rotate_images BOOLEAN DEFAULT TRUE,
    enable_image_preprocessing BOOLEAN DEFAULT TRUE,
    search_results_per_page INT DEFAULT 25,
    search_snippet_length INT DEFAULT 200,
    fuzzy_search_threshold REAL DEFAULT 0.8,
    retention_days INT,
    enable_auto_cleanup BOOLEAN DEFAULT FALSE,
    enable_compression BOOLEAN DEFAULT FALSE,
    memory_limit_mb INT DEFAULT 512,
    cpu_priority VARCHAR(10) DEFAULT 'normal',
    enable_background_ocr BOOLEAN DEFAULT TRUE,
    ocr_page_segmentation_mode INT DEFAULT 3,
    ocr_engine_mode INT DEFAULT 3,
    ocr_min_confidence REAL DEFAULT 30.0,
    ocr_dpi INT DEFAULT 300,
    ocr_enhance_contrast BOOLEAN DEFAULT true,
    ocr_remove_noise BOOLEAN DEFAULT true,
    ocr_detect_orientation BOOLEAN DEFAULT true,
    ocr_whitelist_chars TEXT,
    ocr_blacklist_chars TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);