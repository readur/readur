// Re-export shim: the thumbnail now lives in features/document. Kept for importers outside that
// feature (DocumentsPage); remove once they import the new path.
export { DocumentThumbnail as default } from '../features/document/DocumentThumbnail';
export type { DocumentThumbnailProps } from '../features/document/DocumentThumbnail';
