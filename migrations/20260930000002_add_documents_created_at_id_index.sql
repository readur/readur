-- Supports the default document list order (newest first, id as tiebreaker)
CREATE INDEX IF NOT EXISTS idx_documents_created_at_id ON documents (created_at DESC, id DESC);
