-- System labels go away. The five shared, read-only labels every user saw (Important, To Review,
-- Archive, Work, Personal) become ordinary labels each user owns, so they can be renamed,
-- recoloured, taken off documents and deleted like any other.
--
-- Safe to run on a database that already went through it: the data steps only run while the
-- is_system column still exists, and the schema steps use IF EXISTS.

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'labels' AND column_name = 'is_system'
    ) THEN
        -- 1. Every user gets their own copy of each shared label. A user who already owns a label
        --    with the same name keeps that one instead.
        INSERT INTO labels (user_id, name, description, color, background_color, icon, created_at, updated_at)
        SELECT u.id, s.name, s.description, s.color, s.background_color, s.icon, NOW(), NOW()
        FROM users u
        CROSS JOIN labels s
        WHERE s.is_system = TRUE
        ON CONFLICT (user_id, name) DO NOTHING;

        -- 2. Move each tag from the shared label to the owner's label of the same name. Only a
        --    document's owner can tag it, so the owner's copy is always the right one.
        INSERT INTO document_labels (document_id, label_id, created_at, assigned_by)
        SELECT dl.document_id, own.id, dl.created_at, dl.assigned_by
        FROM document_labels dl
        JOIN labels s ON s.id = dl.label_id AND s.is_system = TRUE
        JOIN documents d ON d.id = dl.document_id
        JOIN labels own ON own.user_id = d.user_id AND own.name = s.name
        ON CONFLICT (document_id, label_id) DO NOTHING;

        INSERT INTO source_labels (source_id, label_id, created_at, assigned_by)
        SELECT sl.source_id, own.id, sl.created_at, sl.assigned_by
        FROM source_labels sl
        JOIN labels s ON s.id = sl.label_id AND s.is_system = TRUE
        JOIN sources src ON src.id = sl.source_id
        JOIN labels own ON own.user_id = src.user_id AND own.name = s.name
        ON CONFLICT (source_id, label_id) DO NOTHING;

        -- 3. The shared labels go; their old tags (already copied) go with them.
        DELETE FROM labels WHERE is_system = TRUE;
    END IF;
END $$;

-- 4. Nothing is shared any more.
DROP INDEX IF EXISTS idx_system_labels_unique_name;
DROP INDEX IF EXISTS idx_labels_is_system;
ALTER TABLE labels DROP COLUMN IF EXISTS is_system;
ALTER TABLE labels ALTER COLUMN user_id SET NOT NULL;
