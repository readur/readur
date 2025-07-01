#!/usr/bin/env python3
"""
Create proper test PDFs for debugging OCR word counting issues.
"""

try:
    from reportlab.pdfgen import canvas
    from reportlab.lib.pagesizes import letter
    import os
except ImportError:
    print("reportlab not installed. Trying alternative method...")
    # Alternative: create simple text files for testing
    import os
    
    def create_simple_test_files():
        """Create simple text files as a fallback"""
        test_dir = "tests/test_pdfs"
        os.makedirs(test_dir, exist_ok=True)
        
        # Test cases that would be similar to PDF extraction results
        test_cases = [
            ("normal_spacing.txt", "This is a normal document with proper word spacing and punctuation."),
            ("soclogix_sample.txt", "SOCLogix Non-Disclosure Agreement\nThis agreement is entered into between SOCLogix and the recipient for the purpose of protecting confidential information."),
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
    
    # Test case 1: Normal spacing (like SOCLogix NDA)
    pdf_path = f"{test_dir}/soclogix_nda_realistic.pdf"
    c = canvas.Canvas(pdf_path, pagesize=letter)
    width, height = letter
    
    # Add text with normal spacing
    c.setFont("Helvetica", 12)
    y_position = height - 100
    
    lines = [
        "SOCLogix Non-Disclosure Agreement",
        "",
        "This agreement is entered into between SOCLogix and the recipient",
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
    
    print("\nAll test PDFs created successfully!")
    return True

if __name__ == "__main__":
    create_test_pdfs()