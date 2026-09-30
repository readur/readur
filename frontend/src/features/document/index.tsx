// Lazy route entry for /documents/:id. Must default-export the page component.
// Named exports are the pieces other features reuse (Library row thumbnail and share dialog);
// import them from the deep paths instead if pulling the page module in is a concern.
export { default } from './DocumentPage';
export { DocumentThumbnail } from './DocumentThumbnail';
export type { DocumentThumbnailProps } from './DocumentThumbnail';
export { SharedLinksDialog } from './sharing/SharedLinksDialog';
export type { SharedLinksDialogProps } from './sharing/SharedLinksDialog';
