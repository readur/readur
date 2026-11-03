-- Source sync looks up each listed file by (source_id, source_path) before
-- downloading it, to skip files that are already synced unchanged.
CREATE INDEX IF NOT EXISTS idx_documents_source_id_source_path
    ON documents (source_id, source_path)
    WHERE source_id IS NOT NULL;
