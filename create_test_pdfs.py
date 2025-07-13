#!/usr/bin/env python3
"""
Create proper test PDFs for debugging OCR word counting issues.
"""

import os

try:
    from reportlab.pdfgen import canvas
    from reportlab.lib.pagesizes import letter
except ImportError:
    print("reportlab not installed. Trying alternative method...")
    # Alternative: create simple text files for testing
    
    def create_simple_test_files():
        """Create simple text files as a fallback"""
        test_dir = "tests/test_pdfs"
        os.makedirs(test_dir, exist_ok=True)
        
        # Test cases that would be similar to PDF extraction results
        test_cases = [
            ("normal_spacing.txt", "This is a normal document with proper word spacing and punctuation."),
            ("acme_sample.txt", "ACME Non-Disclosure Agreement\nThis agreement is entered into between ACME and the recipient for the purpose of protecting confidential information."),
            ("multiline_text.txt", "Line one with several words\nLine two with more content\nLine three continues the pattern\nFinal line ends the document"),
            ("mixed_content.txt", "Document with numbers 123 and symbols @#$ mixed with normal text."),
            ("special_chars.txt", "Text with special characters: café naïve résumé — and 'quotes' • bullets"),
        ]
        
        for filename, content in test_cases:
            with open(f"{test_dir}/{filename}", "w", encoding="utf-8") as f:
                f.write(content)
        
        print("Created simple text files for testing")
        return True
    
    if not create_simple_test_files():
        exit(1)
    exit(0)

def create_test_pdfs():
    """Create proper test PDFs using reportlab"""
    test_dir = "tests/test_pdfs"
    os.makedirs(test_dir, exist_ok=True)
    
    # Test case 1: Normal spacing (like ACME NDA)
    pdf_path = f"{test_dir}/acme_nda_realistic.pdf"
    c = canvas.Canvas(pdf_path, pagesize=letter)
    width, height = letter
    
    # Add text with normal spacing
    c.setFont("Helvetica", 12)
    y_position = height - 100
    
    lines = [
        "ACME Non-Disclosure Agreement",
        "",
        "This agreement is entered into between ACME and the recipient",
        "for the purpose of protecting confidential information.",
        "",
        "The recipient agrees to maintain strict confidentiality",
        "regarding all proprietary information disclosed.",
        "",
        "This includes but is not limited to technical specifications,",
        "business plans, customer lists, and financial data.",
        "",
        "Any breach of this agreement may result in legal action.",
        "The agreement remains in effect for a period of five years.",
    ]
    
    for line in lines:
        if line:  # Skip empty lines for positioning
            c.drawString(72, y_position, line)
        y_position -= 20
    
    c.save()
    print(f"Created: {pdf_path}")
    
    # Test case 2: Multi-page document
    pdf_path = f"{test_dir}/multipage_realistic.pdf"
    c = canvas.Canvas(pdf_path, pagesize=letter)
    
    # Page 1
    c.setFont("Helvetica", 12)
    y_position = height - 100
    
    page1_lines = [
        "Page 1: Document with Multiple Pages",
        "",
        "This is the first page of a multi-page document.",
        "It contains multiple sentences with proper spacing.",
        "Each line should be counted as separate words.",
        "Word boundaries are clearly defined with spaces.",
        "",
        "Numbers like 123, 456, and 789 should also count.",
        "Punctuation marks help separate thoughts.",
        "Total words on this page should be easily counted.",
    ]
    
    for line in page1_lines:
        if line:
            c.drawString(72, y_position, line)
        y_position -= 20
    
    # Start new page
    c.showPage()
    y_position = height - 100
    
    page2_lines = [
        "Page 2: Continuing from Previous Page",
        "",
        "This page also has normal text formatting.",
        "Word counting should work correctly here too.",
        "Mixed content: ABC123 def456 GHI789 works fine.",
        "",
        "Special characters like café, naïve, and résumé",
        "should also be handled properly by the extraction.",
        "",
        "End of document with proper word boundaries.",
    ]
    
    for line in page2_lines:
        if line:
            c.drawString(72, y_position, line)
        y_position -= 20
    
    c.save()
    print(f"Created: {pdf_path}")
    
    # Test case 3: Document with problematic patterns
    pdf_path = f"{test_dir}/edge_cases_realistic.pdf"
    c = canvas.Canvas(pdf_path, pagesize=letter)
    c.setFont("Helvetica", 12)
    y_position = height - 100
    
    edge_case_lines = [
        "Edge Cases for Word Counting",
        "",
        "Normal text with proper spacing works fine.",
        "TextWithoutSpacesButCamelCase should be detected.",
        "ALLCAPSTEXT might be problematic.",
        "mixed123CASE456text789 has transitions.",
        "",
        "Punctuation!!! should not count as words.",
        "But text-with-hyphens should count properly.",
        "Email@example.com and URLs http://test.com too.",
        "",
        "End with normal text to verify counting.",
    ]
    
    for line in edge_case_lines:
        if line:
            c.drawString(72, y_position, line)
        y_position -= 20
    
    c.save()
    print(f"Created: {pdf_path}")
    
    # Test case 4: Small file (< 1MB)
    pdf_path = f"{test_dir}/small_file.pdf"
    c = canvas.Canvas(pdf_path, pagesize=letter)
    c.setFont("Helvetica", 12)
    y_position = height - 100
    
    small_lines = [
        "Small Test Document",
        "",
        "This is a small document for testing.",
        "It should be under 1MB in size.",
        "Perfect for basic upload testing.",
    ]
    
    for line in small_lines:
        if line:
            c.drawString(72, y_position, line)
        y_position -= 20
    
    c.save()
    print(f"Created: {pdf_path}")
    
    # Test case 5: Medium file (2-10MB) - Create with repeated content
    pdf_path = f"{test_dir}/medium_file.pdf"
    c = canvas.Canvas(pdf_path, pagesize=letter)
    c.setFont("Helvetica", 8)
    
    # Create a 5MB file by adding many pages with lots of text
    repeated_text = "This is repeated content to make the file larger and test medium file uploads. " * 15
    for page_num in range(300):  # More pages
        y_position = height - 30
        c.drawString(72, y_position, f"Page {page_num + 1}: Medium Size Test Document for Upload Testing")
        y_position -= 15
        
        # Add much more content per page
        for i in range(50):  # More lines per page
            if y_position < 30:
                break
            # Use longer text to increase file size
            line_text = f"Line {i + 1}: {repeated_text}"[:120]
            c.drawString(72, y_position, line_text)
            y_position -= 12
        
        if page_num < 299:
            c.showPage()
    
    c.save()
    print(f"Created: {pdf_path}")
    
    # Test case 6: Large file (10-49MB) - Create with even more content
    pdf_path = f"{test_dir}/large_file.pdf"
    c = canvas.Canvas(pdf_path, pagesize=letter)
    c.setFont("Helvetica", 6)  # Very small font to fit more
    
    # Add many pages with very dense content to reach ~25MB
    dense_text = "Dense content for large file testing with lots of characters to increase file size significantly. " * 25
    for page_num in range(800):  # Many more pages
        y_position = height - 20
        c.drawString(72, y_position, f"Page {page_num + 1}: Large File Test Document for Upload Testing - Should be around 25MB")
        y_position -= 12
        
        # Add extremely dense content
        for i in range(80):  # Maximum lines per page
            if y_position < 20:
                break
            line_text = f"{i + 1}: {dense_text}"[:150]  # Long lines
            c.drawString(72, y_position, line_text)
            y_position -= 8  # Tight line spacing
        
        if page_num < 799:
            c.showPage()
    
    c.save()
    print(f"Created: {pdf_path}")
    
    # Test case 7: Oversized file (> 50MB) - Should fail upload
    pdf_path = f"{test_dir}/oversized_file.pdf"
    c = canvas.Canvas(pdf_path, pagesize=letter)
    c.setFont("Helvetica", 5)  # Very small font
    
    # Create an extremely large file that exceeds the 50MB limit
    massive_text = "This file is designed to exceed the 50MB upload limit and should fail gracefully. " * 50
    for page_num in range(1500):  # Many pages to exceed 50MB
        y_position = height - 15
        c.drawString(72, y_position, f"Page {page_num + 1}: Oversized Test Document - Should Fail Upload (Target: >50MB)")
        y_position -= 10
        
        for i in range(100):  # Maximum lines per page
            if y_position < 15:
                break
            line_text = f"{i + 1}: {massive_text}"[:160]  # Very long lines
            c.drawString(72, y_position, line_text)
            y_position -= 7  # Very tight spacing
        
        if page_num < 1499:
            c.showPage()
    
    c.save()
    print(f"Created: {pdf_path}")
    
    print("\n📊 File Size Summary:")
    print("=" * 50)
    
    # Check actual file sizes
    test_files = [
        "small_file.pdf",
        "medium_file.pdf", 
        "large_file.pdf",
        "oversized_file.pdf",
        "acme_nda_realistic.pdf",
        "multipage_realistic.pdf",
        "edge_cases_realistic.pdf"
    ]
    
    for filename in test_files:
        filepath = f"{test_dir}/{filename}"
        if os.path.exists(filepath):
            size_bytes = os.path.getsize(filepath)
            size_mb = size_bytes / (1024 * 1024)
            print(f"📄 {filename}: {size_mb:.2f} MB ({size_bytes:,} bytes)")
    
    print("\nAll test PDFs created successfully!")
    return True

if __name__ == "__main__":
    create_test_pdfs()