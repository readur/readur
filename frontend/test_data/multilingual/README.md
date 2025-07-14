# Multilingual OCR Test Files

This directory contains test files for validating the multiple OCR language capabilities of Readur.

## Test Files

### Spanish Test Files
- **`spanish_test.pdf`** - Basic Spanish document with common words, accents, and phrases
- **`spanish_complex.pdf`** - Complex Spanish document with special characters (ñ, ü, ¿, ¡)

### English Test Files  
- **`english_test.pdf`** - Basic English document with common words and technical terms
- **`english_complex.pdf`** - Complex English document with contractions, hyphens, and abbreviations

### Mixed Language Test Files
- **`mixed_language_test.pdf`** - Document containing both Spanish and English text sections

## Expected OCR Content

### Spanish Content Keywords
- español, documento, reconocimiento
- café, niño, comunicación, corazón
- también, habitación, compañía
- informática, educación, investigación

### English Content Keywords
- English, document, recognition
- technology, computer, software, hardware
- testing, validation, verification, quality

### Mixed Content
Both Spanish and English keywords should be recognized in the mixed language document.

## Usage in E2E Tests

These files are used by the `ocr-multiple-languages.spec.ts` test suite to validate:

1. **Language Selection**: Testing the OCR language selector component
2. **Document Upload**: Uploading documents with specific language preferences
3. **OCR Processing**: Validating OCR results contain expected language-specific content
4. **Language Persistence**: Ensuring language preferences are saved across sessions
5. **Retry Functionality**: Testing OCR retry with different languages
6. **Error Handling**: Testing graceful fallback behavior

## Test Languages

- **Spanish (spa)**: Primary test language with accents and special characters
- **English (eng)**: Secondary test language with technical terminology
- **Auto-detect**: Testing automatic language detection

## File Creation

These files were created using the `create_multilingual_test_pdfs.py` script in the repository root.

To regenerate the test files:

```bash
python3 create_multilingual_test_pdfs.py
```

## OCR Language Testing Workflow

1. Set language preference in Settings page
2. Upload test document with specific language content
3. Wait for OCR processing to complete
4. Validate OCR results contain expected keywords
5. Test retry functionality with different languages
6. Verify bulk operations work with multiple languages

## Expected Test Results

When OCR is configured correctly for Spanish (`spa`):
- Spanish documents should have high recognition accuracy for accented characters
- Phrases like "Hola mundo", "este es un documento", "en español" should be recognized
- Special characters (ñ, ü, ¿, ¡) should be preserved

When OCR is configured correctly for English (`eng`):
- English documents should have high recognition accuracy
- Technical terms and abbreviations should be recognized
- Phrases like "Hello world", "this is an English", "document" should be recognized

## Troubleshooting

If tests fail:

1. **Check Tesseract Installation**: Ensure Spanish language pack is installed
   ```bash
   # Ubuntu/Debian
   sudo apt-get install tesseract-ocr-spa
   
   # macOS
   brew install tesseract-lang
   ```

2. **Verify Language Availability**: Check `/api/ocr/languages` endpoint returns Spanish and English

3. **File Paths**: Ensure test files exist in the correct directory structure

4. **OCR Processing Time**: Allow sufficient timeout (120s) for OCR processing to complete