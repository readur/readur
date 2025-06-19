-- Create labels table
CREATE TABLE labels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    color VARCHAR(7) NOT NULL DEFAULT '#0969da', -- hex color code
    background_color VARCHAR(7), -- optional background color
    icon VARCHAR(100), -- optional icon identifier
    is_system BOOLEAN NOT NULL DEFAULT FALSE, -- system labels vs user labels
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(user_id, name) -- prevent duplicate label names per user
);

-- Create document_labels junction table
CREATE TABLE document_labels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    label_id UUID NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(document_id, label_id) -- prevent duplicate assignments
);

-- Create source_labels junction table
CREATE TABLE source_labels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    label_id UUID NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(source_id, label_id) -- prevent duplicate assignments
);

-- Create indexes for performance
CREATE INDEX idx_labels_user_id ON labels(user_id);
CREATE INDEX idx_labels_name ON labels(name);
CREATE INDEX idx_labels_is_system ON labels(is_system);
CREATE INDEX idx_document_labels_document_id ON document_labels(document_id);
CREATE INDEX idx_document_labels_label_id ON document_labels(label_id);
CREATE INDEX idx_source_labels_source_id ON source_labels(source_id);
CREATE INDEX idx_source_labels_label_id ON source_labels(label_id);

-- Create updated_at trigger for labels table
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_labels_updated_at BEFORE UPDATE ON labels
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Insert some default system labels
INSERT INTO labels (id, user_id, name, description, color, is_system, created_at, updated_at) VALUES
    ('00000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000000', 'Important', 'High priority documents', '#d73a49', TRUE, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000002', '00000000-0000-0000-0000-000000000000', 'To Review', 'Documents that need review', '#f66a0a', TRUE, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000003', '00000000-0000-0000-0000-000000000000', 'Archive', 'Archived documents', '#6f42c1', TRUE, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000004', '00000000-0000-0000-0000-000000000000', 'Work', 'Work-related documents', '#0969da', TRUE, NOW(), NOW()),
    ('00000000-0000-0000-0000-000000000005', '00000000-0000-0000-0000-000000000000', 'Personal', 'Personal documents', '#1a7f37', TRUE, NOW(), NOW());