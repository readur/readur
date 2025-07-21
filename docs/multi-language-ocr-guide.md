# Multi-Language OCR Guide

Readur supports powerful multi-language OCR capabilities that allow you to process documents in multiple languages simultaneously for optimal text extraction accuracy.

## 🌍 Overview

The multi-language OCR system allows you to:
- **Process documents in up to 4 languages simultaneously** for best results
- **Set preferred languages** that apply to all your document uploads
- **Retry failed OCR** with different language combinations
- **Automatically optimize** text extraction by using multiple language models

## 🚀 Getting Started

### Setting Your Language Preferences

1. **Navigate to Settings** in your account
2. **Select OCR Languages** section
3. **Choose up to 4 preferred languages** - these will be used for all new uploads
4. **Set a primary language** - this language gets processing priority
5. **Save your preferences**

**Example preferred language setup:**
- Primary: English (`eng`)
- Additional: Spanish (`spa`), French (`fra`)
- Result: Documents processed with English priority, plus Spanish and French recognition

### Language Selection During Upload

When uploading documents, you can:

1. **Use your default preferences** - no action needed
2. **Override for specific documents:**
   - Click the language selector in the upload area
   - Choose different languages for this upload session
   - These languages will be applied to all files in the current upload

## 📋 Available Languages

Readur supports 67+ languages including:

### Major World Languages
- **English** (`eng`) - Default and most reliable
- **Spanish** (`spa`) - Excellent accuracy
- **French** (`fra`) - High quality results
- **German** (`deu`) - Strong performance
- **Italian** (`ita`) - Good accuracy
- **Portuguese** (`por`) - Reliable processing
- **Russian** (`rus`) - Solid results

### Asian Languages  
- **Chinese Simplified** (`chi_sim`)
- **Chinese Traditional** (`chi_tra`)
- **Japanese** (`jpn`)
- **Korean** (`kor`)
- **Hindi** (`hin`)
- **Thai** (`tha`)
- **Vietnamese** (`vie`)

### European Languages
- **Dutch** (`nld`)
- **Swedish** (`swe`)
- **Norwegian** (`nor`)
- **Danish** (`dan`)
- **Finnish** (`fin`)
- **Polish** (`pol`)
- **Czech** (`ces`)

### And Many More
Including Arabic (`ara`), Hebrew (`heb`), Turkish (`tur`), and dozens of other languages.

> **Tip:** For the complete list of available languages, visit the OCR Languages page in your settings or call the API endpoint: `GET /api/ocr/languages`

## 🛠️ Using the API

### Get Available Languages
```bash
curl -H "Authorization: Bearer YOUR_TOKEN" \
     https://your-readur-instance.com/api/ocr/languages
```

**Response:**
```json
{
  "available_languages": [
    {
      "code": "eng",
      "name": "English",
      "installed": true
    },
    {
      "code": "spa", 
      "name": "Spanish",
      "installed": true
    }
  ],
  "current_user_language": "eng"
}
```

### Update Language Preferences
```bash
curl -X PUT \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{
       "preferred_languages": ["eng", "spa", "fra"],
       "primary_language": "eng"
     }' \
     https://your-readur-instance.com/api/settings
```

### Retry OCR with Different Languages
```bash
curl -X POST \
     -H "Authorization: Bearer YOUR_TOKEN" \
     -H "Content-Type: application/json" \
     -d '{
       "languages": ["eng", "deu"]
     }' \
     https://your-readur-instance.com/api/documents/DOCUMENT_ID/ocr/retry
```

## 🎯 Best Practices

### Language Selection Strategy

**For Mixed-Language Documents:**
- Choose 2-3 languages that appear in your document
- Always include English as a fallback (most reliable)
- Put the dominant language first as your primary language

**Examples:**
- **Business document with English/Spanish:** `["eng", "spa"]`
- **European legal document:** `["eng", "fra", "deu"]`
- **Academic paper with multiple references:** `["eng", "spa", "ita"]`

### Performance Optimization

**Do:**
- ✅ Limit to 2-4 languages for best performance
- ✅ Include English when processing mixed content
- ✅ Use specific language combinations for consistent document types
- ✅ Set realistic expectations for complex multilingual documents

**Don't:**
- ❌ Select languages not present in your documents
- ❌ Use more than 4 languages simultaneously
- ❌ Expect perfect results with very low-quality scans
- ❌ Mix completely unrelated language families unnecessarily

## 🔄 Retrying OCR Processing

If OCR results are poor, you can retry with different languages:

### Via Web Interface
1. **Navigate to the document** with poor OCR results
2. **Click "Retry OCR"** button
3. **Select different languages** that better match your document
4. **Start retry process**

### Common Retry Scenarios

**Scenario 1: Wrong Language Detected**
- Original: English-only processing of Spanish document
- Solution: Retry with `["spa", "eng"]`

**Scenario 2: Mixed Language Document**
- Original: Single language processing
- Solution: Add 2-3 relevant languages

**Scenario 3: Poor Quality Scan**
- Original: Fast processing with limited languages
- Solution: Try with primary language + English fallback

## 📊 Monitoring OCR Results

### Understanding OCR Confidence
- **90%+** - Excellent results, high accuracy
- **70-89%** - Good results, minor errors possible  
- **50-69%** - Moderate results, review recommended
- **Below 50%** - Poor results, consider retry with different languages

### Language-Specific Performance
Different languages have varying accuracy rates:
- **Latin-based scripts** (English, Spanish, French): Highest accuracy
- **Germanic languages** (German, Dutch): Very good accuracy
- **Asian languages** (Chinese, Japanese): Good accuracy with proper font recognition
- **Arabic/Hebrew scripts**: Moderate accuracy, depends on text quality

## 🐛 Troubleshooting

### Common Issues

**Problem:** "Language not available" error
**Solution:** 
- Check language code spelling (e.g., `eng` not `english`)
- Verify language is installed on the server
- Contact administrator if language should be available

**Problem:** Poor OCR results despite correct language
**Solutions:**
- Ensure document scan quality is sufficient (300+ DPI recommended)
- Try adding English as a fallback language
- Consider document preprocessing (contrast, rotation correction)
- Retry with fewer languages for better performance

**Problem:** Slow processing with multiple languages  
**Solutions:**
- Reduce number of selected languages to 2-3
- Use languages only present in your document
- Consider processing during off-peak hours

### Getting Help

If you're experiencing issues:

1. **Check the OCR Health page** - `GET /api/ocr/health`
2. **Review your language selection** - ensure languages match document content
3. **Try with English fallback** - adds reliability to processing
4. **Contact support** with document ID and language combination used

## 🔮 Advanced Features

### Planned Enhancements
- **Auto-language detection**: Automatic suggestion of optimal language combinations
- **Custom language models**: Upload your own specialized language data
- **Batch language updates**: Change languages for multiple documents at once
- **Language-specific confidence thresholds**: Fine-tune accuracy requirements per language

### Integration Options
The multi-language OCR system integrates with:
- **Document management workflows**
- **Automated processing pipelines**  
- **Third-party applications via REST API**
- **Webhook notifications for completion**

## 📚 Additional Resources

- **API Documentation**: Complete endpoint reference
- **Language Codes Reference**: Full list of supported language codes
- **Performance Guidelines**: Optimization recommendations
- **Migration Guide**: Upgrading from single-language setup

---

**Need Help?** Contact support or check the system health dashboard for real-time OCR capability status.