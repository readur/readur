-- Add OCR queue table for robust processing
CREATE TABLE IF NOT EXISTS ocr_queue (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID REFERENCES documents(id) ON DELETE CASCADE,
    status VARCHAR(20) DEFAULT 'pending',
    priority INT DEFAULT 5,
    attempts INT DEFAULT 0,
    max_attempts INT DEFAULT 3,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    worker_id VARCHAR(100),
    processing_time_ms INT,
    file_size BIGINT,
    CONSTRAINT check_status CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'cancelled'))
);

CREATE INDEX IF NOT EXISTS idx_ocr_queue_status ON ocr_queue(status, priority DESC, created_at);

CREATE INDEX IF NOT EXISTS idx_ocr_queue_document_id ON ocr_queue(document_id);

CREATE INDEX IF NOT EXISTS idx_ocr_queue_worker ON ocr_queue(worker_id) WHERE status = 'processing';

CREATE INDEX IF NOT EXISTS idx_ocr_queue_created_at ON ocr_queue(created_at) WHERE status = 'pending';

ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_status VARCHAR(20) DEFAULT 'pending';

ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_error TEXT;

ALTER TABLE documents ADD COLUMN IF NOT EXISTS ocr_completed_at TIMESTAMPTZ;

CREATE TABLE IF NOT EXISTS ocr_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    date DATE DEFAULT CURRENT_DATE,
    hour INT DEFAULT EXTRACT(HOUR FROM NOW()),
    total_processed INT DEFAULT 0,
    total_failed INT DEFAULT 0,
    total_retried INT DEFAULT 0,
    avg_processing_time_ms INT,
    max_processing_time_ms INT,
    min_processing_time_ms INT,
    queue_depth INT,
    active_workers INT,
    UNIQUE(date, hour)
);