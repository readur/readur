-- Add assigned_by column to document_labels table
ALTER TABLE document_labels ADD COLUMN assigned_by UUID REFERENCES users(id) ON DELETE SET NULL;

-- Add assigned_by column to source_labels table for consistency
ALTER TABLE source_labels ADD COLUMN assigned_by UUID REFERENCES users(id) ON DELETE SET NULL;

-- Add indexes for performance
CREATE INDEX idx_document_labels_assigned_by ON document_labels(assigned_by);
CREATE INDEX idx_source_labels_assigned_by ON source_labels(assigned_by);