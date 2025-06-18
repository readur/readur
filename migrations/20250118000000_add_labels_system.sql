-- Migration: Add enhanced label system
-- Description: Creates tables for a GitHub Issues-style label system with colors and icons

-- Create labels table
CREATE TABLE IF NOT EXISTS labels (
    id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(50) NOT NULL,
    description TEXT,
    color VARCHAR(7) NOT NULL DEFAULT '#0969da', -- Hex color (GitHub blue)
    background_color VARCHAR(7) DEFAULT NULL, -- Optional background color for gradient effects
    icon VARCHAR(50), -- Icon identifier (e.g., 'bug', 'enhancement', 'documentation')
    is_system BOOLEAN DEFAULT FALSE, -- System labels vs user labels
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(user_id, name)
);

-- Create document_labels junction table
CREATE TABLE IF NOT EXISTS document_labels (
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    label_id UUID NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    assigned_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    assigned_by UUID REFERENCES users(id),
    PRIMARY KEY (document_id, label_id)
);

-- Create source_labels junction table (for labeling sources)
CREATE TABLE IF NOT EXISTS source_labels (
    source_id UUID NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
    label_id UUID NOT NULL REFERENCES labels(id) ON DELETE CASCADE,
    assigned_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    assigned_by UUID REFERENCES users(id),
    PRIMARY KEY (source_id, label_id)
);

-- Create indexes for performance
CREATE INDEX idx_labels_user_id ON labels(user_id);
CREATE INDEX idx_labels_name ON labels(name);
CREATE INDEX idx_labels_is_system ON labels(is_system);
CREATE INDEX idx_document_labels_document_id ON document_labels(document_id);
CREATE INDEX idx_document_labels_label_id ON document_labels(label_id);
CREATE INDEX idx_source_labels_source_id ON source_labels(source_id);
CREATE INDEX idx_source_labels_label_id ON source_labels(label_id);

-- Function to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Trigger to auto-update updated_at
CREATE TRIGGER update_labels_updated_at BEFORE UPDATE ON labels
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Function to get all labels for a document
CREATE OR REPLACE FUNCTION get_document_labels(p_document_id UUID)
RETURNS TABLE(
    id UUID,
    name VARCHAR(50),
    description TEXT,
    color VARCHAR(7),
    background_color VARCHAR(7),
    icon VARCHAR(50),
    is_system BOOLEAN
) AS $$
BEGIN
    RETURN QUERY
    SELECT l.id, l.name, l.description, l.color, l.background_color, l.icon, l.is_system
    FROM labels l
    INNER JOIN document_labels dl ON l.id = dl.label_id
    WHERE dl.document_id = p_document_id
    ORDER BY l.name;
END;
$$ LANGUAGE plpgsql;

-- Function to add a label to a document
CREATE OR REPLACE FUNCTION add_document_label(
    p_document_id UUID,
    p_label_id UUID,
    p_user_id UUID
) RETURNS BOOLEAN AS $$
BEGIN
    INSERT INTO document_labels (document_id, label_id, assigned_by)
    VALUES (p_document_id, p_label_id, p_user_id)
    ON CONFLICT (document_id, label_id) DO NOTHING;
    
    RETURN FOUND;
END;
$$ LANGUAGE plpgsql;

-- Function to remove a label from a document
CREATE OR REPLACE FUNCTION remove_document_label(
    p_document_id UUID,
    p_label_id UUID
) RETURNS BOOLEAN AS $$
BEGIN
    DELETE FROM document_labels
    WHERE document_id = p_document_id AND label_id = p_label_id;
    
    RETURN FOUND;
END;
$$ LANGUAGE plpgsql;

-- Function to create or update a label
CREATE OR REPLACE FUNCTION upsert_label(
    p_user_id UUID,
    p_name VARCHAR(50),
    p_description TEXT DEFAULT NULL,
    p_color VARCHAR(7) DEFAULT '#0969da',
    p_background_color VARCHAR(7) DEFAULT NULL,
    p_icon VARCHAR(50) DEFAULT NULL,
    p_is_system BOOLEAN DEFAULT FALSE
) RETURNS UUID AS $$
DECLARE
    v_label_id UUID;
BEGIN
    INSERT INTO labels (user_id, name, description, color, background_color, icon, is_system)
    VALUES (p_user_id, p_name, p_description, p_color, p_background_color, p_icon, p_is_system)
    ON CONFLICT (user_id, name) DO UPDATE
    SET description = EXCLUDED.description,
        color = EXCLUDED.color,
        background_color = EXCLUDED.background_color,
        icon = EXCLUDED.icon,
        updated_at = CURRENT_TIMESTAMP
    RETURNING id INTO v_label_id;
    
    RETURN v_label_id;
END;
$$ LANGUAGE plpgsql;

-- Function to get label usage count
CREATE OR REPLACE FUNCTION get_label_usage_counts(p_user_id UUID)
RETURNS TABLE(
    label_id UUID,
    name VARCHAR(50),
    color VARCHAR(7),
    icon VARCHAR(50),
    document_count BIGINT,
    source_count BIGINT
) AS $$
BEGIN
    RETURN QUERY
    SELECT 
        l.id,
        l.name,
        l.color,
        l.icon,
        COUNT(DISTINCT dl.document_id) as document_count,
        COUNT(DISTINCT sl.source_id) as source_count
    FROM labels l
    LEFT JOIN document_labels dl ON l.id = dl.label_id
    LEFT JOIN source_labels sl ON l.id = sl.label_id
    WHERE l.user_id = p_user_id OR l.is_system = TRUE
    GROUP BY l.id, l.name, l.color, l.icon
    ORDER BY l.name;
END;
$$ LANGUAGE plpgsql;

-- Migrate existing tags to labels
-- This creates labels from existing document tags
INSERT INTO labels (user_id, name, color, is_system)
SELECT DISTINCT 
    d.user_id,
    unnest(d.tags) as name,
    '#0969da' as color,
    FALSE as is_system
FROM documents d
WHERE d.tags IS NOT NULL AND array_length(d.tags, 1) > 0
ON CONFLICT (user_id, name) DO NOTHING;

-- Link existing document tags to the new labels
INSERT INTO document_labels (document_id, label_id)
SELECT DISTINCT
    d.id as document_id,
    l.id as label_id
FROM documents d
CROSS JOIN LATERAL unnest(d.tags) AS tag_name
INNER JOIN labels l ON l.name = tag_name AND l.user_id = d.user_id
WHERE d.tags IS NOT NULL AND array_length(d.tags, 1) > 0
ON CONFLICT (document_id, label_id) DO NOTHING;

-- Add some default system labels
INSERT INTO labels (user_id, name, description, color, icon, is_system) VALUES
    ('00000000-0000-0000-0000-000000000000'::UUID, 'Important', 'High priority items', '#d73a49', 'star', TRUE),
    ('00000000-0000-0000-0000-000000000000'::UUID, 'Archive', 'Archived items', '#6e7781', 'archive', TRUE),
    ('00000000-0000-0000-0000-000000000000'::UUID, 'Personal', 'Personal documents', '#0e7c3a', 'user', TRUE),
    ('00000000-0000-0000-0000-000000000000'::UUID, 'Work', 'Work-related documents', '#0969da', 'briefcase', TRUE),
    ('00000000-0000-0000-0000-000000000000'::UUID, 'Receipt', 'Receipts and invoices', '#8250df', 'receipt', TRUE),
    ('00000000-0000-0000-0000-000000000000'::UUID, 'Legal', 'Legal documents', '#a475f9', 'scale', TRUE),
    ('00000000-0000-0000-0000-000000000000'::UUID, 'Medical', 'Medical records', '#1f883d', 'medical', TRUE),
    ('00000000-0000-0000-0000-000000000000'::UUID, 'Financial', 'Financial documents', '#fb8500', 'dollar', TRUE)
ON CONFLICT (user_id, name) DO NOTHING;