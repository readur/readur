#!/usr/bin/env python3
"""
Create test files for metadata extraction testing.
"""

import os
import sys
from pathlib import Path

# Try to import PIL for image creation
try:
    from PIL import Image, ImageDraw, ImageFont
    from PIL.ExifTags import TAGS
    from PIL.ExifTags import GPSTAGS
    PIL_AVAILABLE = True
except ImportError:
    print("PIL not available, skipping image creation with EXIF")
    PIL_AVAILABLE = False

# Try to import reportlab for PDF creation
try:
    from reportlab.pdfgen import canvas
    from reportlab.lib.pagesizes import letter, A4
    from reportlab.pdfbase import pdfmetrics
    from reportlab.pdfbase.ttfonts import TTFont
    REPORTLAB_AVAILABLE = True
except ImportError:
    print("reportlab not available, creating simple PDF-like files")
    REPORTLAB_AVAILABLE = False

def create_test_images():
    """Create test images with various properties."""
    if not PIL_AVAILABLE:
        print("Skipping image creation - PIL not available")
        return
    
    print("Creating test images...")
    
    # 1. Portrait image (100x200)
    img = Image.new('RGB', (100, 200), color='lightblue')
    draw = ImageDraw.Draw(img)
    draw.text((10, 50), "Portrait\n100x200", fill='black')
    img.save('test_files/portrait_100x200.png')
    
    # 2. Landscape image (300x200)
    img = Image.new('RGB', (300, 200), color='lightgreen')
    draw = ImageDraw.Draw(img)
    draw.text((50, 50), "Landscape 300x200", fill='black')
    img.save('test_files/landscape_300x200.png')
    
    # 3. Square image (150x150)
    img = Image.new('RGB', (150, 150), color='lightyellow')
    draw = ImageDraw.Draw(img)
    draw.text((25, 50), "Square\n150x150", fill='black')
    img.save('test_files/square_150x150.png')
    
    # 4. High resolution image (1920x1080)
    img = Image.new('RGB', (1920, 1080), color='lightcoral')
    draw = ImageDraw.Draw(img)
    try:
        font = ImageFont.truetype("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf", 40)
    except:
        font = ImageFont.load_default()
    draw.text((100, 500), "High Resolution\n1920x1080\n2.07 Megapixels", fill='black', font=font)
    img.save('test_files/hires_1920x1080.png')
    
    # 5. Small image (50x50)
    img = Image.new('RGB', (50, 50), color='lightgray')
    img.save('test_files/small_50x50.png')
    
    # 6. JPEG with different color mode
    img = Image.new('RGB', (200, 200), color='purple')
    draw = ImageDraw.Draw(img)
    draw.text((50, 50), "JPEG\nTest", fill='white')
    img.save('test_files/test_image.jpg', 'JPEG')
    
    print("Created test images")

def create_test_pdfs():
    """Create test PDFs with various properties."""
    if not REPORTLAB_AVAILABLE:
        print("Creating simple PDF-like files...")
        # Create simple files that look like PDF headers
        simple_pdfs = [
            ("%PDF-1.4\n1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj\n3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n>>\nendobj\nxref\n0 4\n0000000000 65535 f \n0000000009 00000 n \n0000000074 00000 n \n0000000120 00000 n \ntrailer\n<<\n/Size 4\n/Root 1 0 R\n>>\nstartxref\n179\n%%EOF", "simple_v14.pdf"),
            ("%PDF-1.7\n1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R 4 0 R]\n/Count 2\n>>\nendobj\n3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n>>\nendobj\n4 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n>>\nendobj\nxref\n0 5\ntrailer\n<<\n/Size 5\n/Root 1 0 R\n>>\n%%EOF", "multipage_v17.pdf"),
            ("%PDF-1.5\n1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n/Linearized true\n>>\nendobj\n2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj\n3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/Font 4 0 R\n/Image 5 0 R\n>>\nendobj\nxref\n0 4\ntrailer\n<<\n/Size 4\n/Root 1 0 R\n>>\n%%EOF", "with_fonts_images.pdf"),
        ]
        
        for content, filename in simple_pdfs:
            with open(f'test_files/{filename}', 'wb') as f:
                f.write(content.encode('latin1'))
        print("Created simple PDF-like files")
        return
    
    print("Creating test PDFs with reportlab...")
    
    # 1. Single page PDF v1.4
    c = canvas.Canvas('test_files/single_page_v14.pdf', pagesize=letter)
    c.setTitle("Single Page Test Document")
    c.setAuthor("Test Author")
    c.setSubject("Test Subject")
    c.setCreator("Python reportlab")
    c.setFont("Helvetica", 12)
    c.drawString(100, 750, "Single Page PDF Document")
    c.drawString(100, 700, "This is a test PDF for metadata extraction.")
    c.drawString(100, 650, "It should be detected as PDF version 1.4")
    c.save()
    
    # 2. Multi-page PDF
    c = canvas.Canvas('test_files/multipage_test.pdf', pagesize=A4)
    c.setTitle("Multi-page Test Document")
    # Page 1
    c.setFont("Helvetica", 14)
    c.drawString(100, 800, "Page 1 of Multi-page Document")
    c.drawString(100, 750, "This document has multiple pages.")
    c.showPage()
    # Page 2
    c.drawString(100, 800, "Page 2 of Multi-page Document")
    c.drawString(100, 750, "Second page content here.")
    c.showPage()
    # Page 3
    c.drawString(100, 800, "Page 3 - Final Page")
    c.drawString(100, 750, "Third and final page.")
    c.save()
    
    # 3. PDF with fonts and complex content
    c = canvas.Canvas('test_files/complex_content.pdf', pagesize=letter)
    c.setTitle("Complex PDF with Fonts")
    c.setFont("Helvetica-Bold", 16)
    c.drawString(100, 750, "Document with Multiple Fonts")
    c.setFont("Helvetica", 12)
    c.drawString(100, 700, "This document contains multiple font types.")
    c.setFont("Courier", 10)
    c.drawString(100, 650, "Some monospace text for variety.")
    # Add some graphics/lines
    c.line(100, 600, 500, 600)
    c.rect(100, 550, 200, 30)
    c.save()
    
    print("Created test PDFs")

def create_text_files():
    """Create various text files for testing."""
    print("Creating test text files...")
    
    # 1. Plain text with various content
    content = """This is a comprehensive test document for text metadata extraction.

It contains multiple paragraphs, various types of content, and different characteristics.
Word count: This sentence has exactly seven words counting properly.
Line counting: Each line should be counted separately for accurate statistics.

Unicode content: café, naïve, résumé, piñata, Zürich, москва, 東京, 🎉✨🔥
Numbers and mixed content: 123 ABC def456 GHI789 test@example.com

Special formatting:
- Bulleted lists
- Multiple items
- With various content

The document ends here with a final paragraph."""
    
    with open('test_files/comprehensive_text.txt', 'w', encoding='utf-8') as f:
        f.write(content)
    
    # 2. JSON format text
    json_content = """{
  "document": {
    "title": "Test JSON Document",
    "type": "metadata_test",
    "properties": {
      "word_count": 25,
      "format": "json",
      "encoding": "utf-8"
    },
    "content": [
      "This JSON should be detected as JSON format",
      "It contains structured data in JSON format"
    ]
  }
}"""
    
    with open('test_files/test_format.json', 'w') as f:
        f.write(json_content)
    
    # 3. XML format text
    xml_content = """<?xml version="1.0" encoding="UTF-8"?>
<document type="test">
  <metadata>
    <title>XML Test Document</title>
    <format>xml</format>
    <word_count>15</word_count>
  </metadata>
  <content>
    <section>This XML document should be detected as XML format.</section>
    <section>It contains structured markup for testing.</section>
  </content>
</document>"""
    
    with open('test_files/test_format.xml', 'w') as f:
        f.write(xml_content)
    
    # 4. HTML format text
    html_content = """<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>HTML Test Document</title>
</head>
<body>
    <h1>HTML Test Page</h1>
    <p>This document should be detected as HTML format.</p>
    <p>It contains HTML markup and structure.</p>
    <ul>
        <li>List item one</li>
        <li>List item two</li>
    </ul>
</body>
</html>"""
    
    with open('test_files/test_format.html', 'w') as f:
        f.write(html_content)
    
    # 5. Large text file for performance testing
    large_content = "This is a large text file for testing performance. " * 1000
    large_content += "\nEnd of large file with final line."
    
    with open('test_files/large_text.txt', 'w') as f:
        f.write(large_content)
    
    # 6. ASCII-only text
    ascii_content = """Pure ASCII text document without any Unicode characters.
This file contains only standard ASCII characters from the basic set.
Numbers: 0123456789
Punctuation: .,;:!?'"()-[]{}
All characters should be ASCII-only for testing encoding detection."""
    
    with open('test_files/ascii_only.txt', 'w') as f:
        f.write(ascii_content)
    
    print("Created test text files")

def main():
    """Create all test files."""
    # Ensure test_files directory exists
    os.makedirs('test_files', exist_ok=True)
    
    print("Creating test files for metadata extraction testing...")
    
    create_text_files()
    create_test_images()
    create_test_pdfs()
    
    print("\nAll test files created successfully!")
    print("Files created in test_files/ directory:")
    
    # List all created files
    test_files = sorted(Path('test_files').glob('*'))
    for file_path in test_files:
        if file_path.is_file() and not file_path.name.endswith('.py'):
            size = file_path.stat().st_size
            print(f"  {file_path.name} ({size} bytes)")

if __name__ == "__main__":
    main()