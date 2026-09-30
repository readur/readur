// Re-export shim: metadata display now lives in features/document. Kept for importers outside
// that feature (DocumentManagementPage); remove once they import the new path.
export { MetadataDisplay as default } from '../features/document/details/MetadataDisplay';
export type { MetadataDisplayProps } from '../features/document/details/MetadataDisplay';
