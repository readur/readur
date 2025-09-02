# Office Document Support

Readur provides comprehensive support for extracting text from Microsoft Office documents, enabling full-text search and content analysis across your document library.

## Supported Formats

### Modern Office Formats (Native Support)
These formats are fully supported without any additional dependencies:

- **DOCX** - Word documents (Office 2007+)
  - Full text extraction from document body
  - Section and paragraph structure preservation
  - Header and footer content extraction
  
- **XLSX** - Excel spreadsheets (Office 2007+)
  - Text extraction from all worksheets
  - Cell content with proper formatting
  - Sheet names and structure preservation

### Legacy Office Formats (External Tools Required)
These older formats require external tools for text extraction:

- **DOC** - Legacy Word documents (Office 97-2003)
  - Requires `antiword`, `catdoc`, or `wvText`
  - Binary format parsing via external tools
  
- **XLS** - Legacy Excel spreadsheets (Office 97-2003)
  - Currently returns an error suggesting conversion to XLSX

## Installation

### Docker Installation
The official Docker image includes all necessary dependencies:

```bash
docker pull readur/readur:latest
```

The Docker image includes `antiword` and `catdoc` pre-installed for legacy DOC support.

### Manual Installation

#### For Modern Formats (DOCX, XLSX)
No additional dependencies required - these formats are parsed using built-in XML processing.

#### For Legacy DOC Files
Install one of the following tools:

**Ubuntu/Debian:**
```bash
# Option 1: antiword (recommended, lightweight)
sudo apt-get install antiword

# Option 2: catdoc (good alternative)
sudo apt-get install catdoc

# Option 3: wv (includes wvText)
sudo apt-get install wv
```

**macOS:**
```bash
# Option 1: antiword
brew install antiword

# Option 2: catdoc
brew install catdoc

# Option 3: wv
brew install wv
```

**Alpine Linux:**
```bash
# Option 1: antiword
apk add antiword

# Option 2: catdoc
apk add catdoc
```

## How It Works

### Modern Office Format Processing (DOCX/XLSX)

1. **ZIP Extraction**: Modern Office files are ZIP archives containing XML files
2. **XML Parsing**: Secure XML parser extracts text content
3. **Content Assembly**: Text from different document parts is assembled
4. **Cleaning**: Excessive whitespace and formatting artifacts are removed

### Legacy DOC Processing

1. **Tool Detection**: System checks for available tools (antiword, catdoc, wvText)
2. **External Processing**: Selected tool converts DOC to plain text
3. **Security Validation**: File paths are validated to prevent injection attacks
4. **Timeout Protection**: 30-second timeout prevents hanging processes
5. **Text Cleaning**: Output is sanitized and normalized

## Configuration

### Timeout Settings
Office document extraction timeout can be configured in user settings:

- **Default**: 120 seconds
- **Range**: 1-600 seconds
- **Applies to**: DOCX and XLSX processing

### Error Handling

When processing fails, Readur provides helpful error messages:

- **Missing Tools**: Instructions for installing required tools
- **File Too Large**: Suggestions for file size reduction
- **Corrupted Files**: Guidance on file repair options
- **Unsupported Formats**: Conversion recommendations

## Security Features

### Built-in Protections

1. **ZIP Bomb Protection**: Limits decompressed size to prevent resource exhaustion
2. **Path Validation**: Prevents directory traversal and injection attacks
3. **XML Security**: Entity expansion and external entity attacks prevented
4. **Process Isolation**: External tools run with limited permissions
5. **Timeout Enforcement**: Prevents infinite processing loops

### File Size Limits

- **Maximum Office Document Size**: 50MB
- **Maximum Decompressed Size**: 500MB (ZIP bomb protection)
- **Compression Ratio Limit**: 100:1

## Performance Considerations

### Processing Speed

Typical extraction times:
- **DOCX (1-10 pages)**: 50-200ms
- **DOCX (100+ pages)**: 500-2000ms
- **XLSX (small)**: 100-300ms
- **XLSX (large)**: 1000-5000ms
- **DOC (via antiword)**: 100-500ms

### Resource Usage

- **Memory**: ~10-50MB per document during processing
- **CPU**: Single-threaded extraction, minimal impact
- **Disk**: Temporary files cleaned automatically

## Troubleshooting

### Common Issues

#### "No DOC extraction tools available"
**Solution**: Install antiword or catdoc as described above.

#### "Document processing timed out"
**Possible causes**:
- Very large or complex document
- Corrupted file structure
- System resource constraints

**Solutions**:
1. Increase timeout in settings
2. Convert to PDF format
3. Split large documents

#### "Document format not supported"
**Affected formats**: PPT, PPTX, and other Office formats

**Solution**: Convert to supported format (PDF, DOCX, TXT)

### Verification

To verify Office document support:

```bash
# Check for DOC support
which antiword || which catdoc || echo "No DOC tools installed"

# Test extraction (Docker)
docker exec readur-container antiword -v

# Test extraction (Manual)
antiword test.doc
```

## Best Practices

1. **Prefer Modern Formats**: Use DOCX over DOC when possible
2. **Convert Legacy Files**: Batch convert DOC to DOCX for better performance
3. **Monitor File Sizes**: Large Office files may need splitting
4. **Regular Updates**: Keep external tools updated for security
5. **Test Extraction**: Verify text extraction quality after setup

## Migration from DOC to DOCX

For better performance and reliability, consider converting legacy DOC files:

### Using LibreOffice (Batch Conversion)
```bash
libreoffice --headless --convert-to docx *.doc
```

### Using Microsoft Word (Windows)
PowerShell script for batch conversion available in `/scripts/convert-doc-to-docx.ps1`

## API Usage

### Upload Office Document
```bash
curl -X POST http://localhost:8000/api/documents/upload \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -F "file=@document.docx"
```

### Check Processing Status
```bash
curl http://localhost:8000/api/documents/{id}/status \
  -H "Authorization: Bearer YOUR_TOKEN"
```

## Future Enhancements

Planned improvements for Office document support:

- [ ] Native DOC parsing (without external tools)
- [ ] PowerPoint (PPTX/PPT) support
- [ ] Table structure preservation
- [ ] Embedded image extraction
- [ ] Style and formatting metadata
- [ ] Track changes and comments extraction

## Related Documentation

- [File Upload Guide](./file-upload-guide.md)
- [OCR Optimization Guide](./dev/OCR_OPTIMIZATION_GUIDE.md)
- [Advanced Search](./advanced-search.md)
- [Configuration Reference](./configuration-reference.md)