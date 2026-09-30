// Re-export shim: the viewer now lives in features/document. Kept for importers outside that
// feature (DocumentManagementPage); remove once they import the new path.
export { DocumentViewer as default } from '../features/document/reading/DocumentViewer';
export type { DocumentViewerProps } from '../features/document/reading/DocumentViewer';
