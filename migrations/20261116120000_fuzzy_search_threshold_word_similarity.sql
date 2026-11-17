-- Fuzzy search now matches word by word (pg_trgm word_similarity of the query
-- against the closest stretch of a document's text) and applies the user's
-- fuzzy_search_threshold, which it previously ignored. On that scale the old
-- default of 0.8 is close to an exact match ("invoce" scores about 0.57
-- against "invoice"), so the default becomes 0.5.
--
-- Rows still holding 0.8 (or NULL) get the new default too: the setting had
-- no effect until now, so 0.8 there is the untouched old default, and keeping
-- it would make fuzzy search reject most typos for every existing user.
-- No new index is needed: the `<%` operator is served by the existing
-- idx_documents_content_trgm (gin_trgm_ops over content || ' ' || ocr_text).

ALTER TABLE settings ALTER COLUMN fuzzy_search_threshold SET DEFAULT 0.5;

UPDATE settings
SET fuzzy_search_threshold = 0.5
WHERE fuzzy_search_threshold IS NULL OR fuzzy_search_threshold = 0.8::real;
