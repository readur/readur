# 📤 Smart File Upload Guide

Readur provides an intuitive drag-and-drop file upload system that supports multiple document formats and batch processing.

## Supported File Types

- **PDF Files** (.pdf) - Direct text extraction and OCR for scanned PDFs
- **Images** (.png, .jpg, .jpeg, .tiff, .bmp, .webp) - Full OCR text extraction
- **Text Files** (.txt, .rtf) - Direct text import
- **Office Documents** (.docx, .doc, .xlsx, .xls, .pptx, .ppt) - Text extraction and OCR

## Upload Methods

### Drag & Drop
1. Navigate to the main dashboard
2. Drag files from your computer directly onto the upload area
3. Multiple files can be selected and dropped simultaneously
4. Progress indicators show upload and processing status

### Browse & Select
1. Click the "Upload Documents" button
2. Use the file browser to select one or multiple files
3. Click "Open" to begin the upload process

## Batch Processing

- Upload multiple files at once for efficient processing
- Each file is processed independently for OCR and text extraction
- Real-time status updates show processing progress
- Failed uploads can be retried individually

## Processing Pipeline

1. **File Validation** - Verify file type and size limits
2. **Storage** - Secure file storage with backup
3. **OCR Processing** - Automatic text extraction using Tesseract
4. **Indexing** - Full-text search indexing in PostgreSQL
5. **Metadata Extraction** - File properties and document information

## Best Practices

- **File Size**: Keep individual files under 50MB for optimal performance
- **File Names**: Use descriptive names for better organization
- **Batch Size**: Upload 10-20 files at once for best performance
- **Network**: Stable internet connection recommended for large uploads

## Troubleshooting

### Upload Fails
- Check file size limits
- Verify file format is supported
- Ensure stable internet connection
- Try uploading fewer files at once

### OCR Issues
- Ensure images have good contrast and resolution
- PDF files may need higher quality scans
- Check the [OCR Optimization Guide](dev/OCR_OPTIMIZATION_GUIDE.md) for advanced tips

## Security

- All uploads are scanned for malicious content
- Files are stored securely with proper access controls
- User permissions apply to all uploaded documents
- Automatic backup ensures data safety