CREATE TABLE document_nodes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    label TEXT NOT NULL,
    name TEXT NOT NULL,
    properties JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE document_edges (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    document_id UUID NOT NULL REFERENCES documents(id) ON DELETE CASCADE,
    source_node_id UUID NOT NULL REFERENCES document_nodes(id) ON DELETE CASCADE,
    target_node_id UUID NOT NULL REFERENCES document_nodes(id) ON DELETE CASCADE,
    relationship TEXT NOT NULL,
    properties JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index for faster lookups
CREATE INDEX idx_document_nodes_document_id ON document_nodes(document_id);
CREATE INDEX idx_document_edges_document_id ON document_edges(document_id);
CREATE INDEX idx_document_edges_source ON document_edges(source_node_id);
CREATE INDEX idx_document_edges_target ON document_edges(target_node_id);
