#!/usr/bin/env python3
"""
Create test PDFs with Spanish and English content for OCR multiple language testing.
"""

import os

try:
    from reportlab.pdfgen import canvas
    from reportlab.lib.pagesizes import letter
    from reportlab.pdfbase import pdfmetrics
    from reportlab.pdfbase.ttfonts import TTFont
except ImportError:
    print("reportlab not installed. Please install it with: pip install reportlab")
    print("Creating simple text files as fallback...")
    
    def create_simple_multilingual_files():
        """Create simple text files as a fallback"""
        test_dir = "frontend/test_data/multilingual"
        os.makedirs(test_dir, exist_ok=True)
        
        # Spanish content
        spanish_content = """Hola mundo, este es un documento en español.
Este documento contiene texto en español para probar el reconocimiento óptico de caracteres.
Las palabras incluyen acentos como café, niño, comunicación y corazón.
También incluye números como 123, 456 y fechas como 15 de marzo de 2024.
El sistema OCR debe reconocer correctamente este contenido en español."""

        # English content
        english_content = """Hello world, this is an English document.
This document contains English text for optical character recognition testing.
The words include common English vocabulary and technical terms.
It also includes numbers like 123, 456 and dates like March 15, 2024.
The OCR system should correctly recognize this English content."""

        # Mixed content
        mixed_content = """Documento bilingüe / Bilingual Document

Sección en español:
Este es un documento que contiene texto en dos idiomas diferentes.
El reconocimiento óptico de caracteres debe manejar ambos idiomas.

English section:
This is a document that contains text in two different languages.
The optical character recognition should handle both languages."""

        with open(f"{test_dir}/spanish_test.txt", "w", encoding="utf-8") as f:
            f.write(spanish_content)
        
        with open(f"{test_dir}/english_test.txt", "w", encoding="utf-8") as f:
            f.write(english_content)
            
        with open(f"{test_dir}/mixed_language_test.txt", "w", encoding="utf-8") as f:
            f.write(mixed_content)
        
        print("Created simple multilingual text files for testing")
        return True
    
    if not create_simple_multilingual_files():
        exit(1)
    exit(0)

def create_multilingual_test_pdfs():
    """Create test PDFs with Spanish and English content"""
    test_dir = "frontend/test_data/multilingual"
    os.makedirs(test_dir, exist_ok=True)
    
    # Spanish test PDF
    pdf_path = f"{test_dir}/spanish_test.pdf"
    c = canvas.Canvas(pdf_path, pagesize=letter)
    width, height = letter
    
    # Spanish content
    c.setFont("Helvetica", 14)
    y_position = height - 80
    
    # Title
    c.drawString(72, y_position, "Documento de Prueba en Español")
    y_position -= 40
    
    c.setFont("Helvetica", 12)
    spanish_lines = [
        "Hola mundo, este es un documento en español.",
        "",
        "Este documento contiene texto en español para probar",
        "el reconocimiento óptico de caracteres (OCR).",
        "",
        "Las palabras incluyen acentos como:",
        "• café, niño, comunicación, corazón",
        "• también, habitación, compañía",
        "• informática, educación, investigación",
        "",
        "Números y fechas en español:",
        "• 123 ciento veintitrés",
        "• 456 cuatrocientos cincuenta y seis", 
        "• 15 de marzo de 2024",
        "• 31 de diciembre de 2023",
        "",
        "Frases comunes:",
        "Por favor, muchas gracias, de nada.",
        "¿Cómo está usted? Muy bien, gracias.",
        "Buenos días, buenas tardes, buenas noches.",
        "",
        "El sistema OCR debe reconocer correctamente",
        "todo este contenido en español, incluyendo",
        "los caracteres especiales y acentos.",
    ]
    
    for line in spanish_lines:
        if line:
            c.drawString(72, y_position, line)
        y_position -= 18
        if y_position < 50:  # Start new page if needed
            c.showPage()
            y_position = height - 50
    
    c.save()
    print(f"Created: {pdf_path}")
    
    # English test PDF
    pdf_path = f"{test_dir}/english_test.pdf"
    c = canvas.Canvas(pdf_path, pagesize=letter)
    
    c.setFont("Helvetica", 14)
    y_position = height - 80
    
    # Title
    c.drawString(72, y_position, "English Test Document")
    y_position -= 40
    
    c.setFont("Helvetica", 12)
    english_lines = [
        "Hello world, this is an English document.",
        "",
        "This document contains English text for testing",
        "optical character recognition (OCR) capabilities.",
        "",
        "Common English words and phrases:",
        "• technology, computer, software, hardware",
        "• document, recognition, character, optical",
        "• testing, validation, verification, quality",
        "",
        "Numbers and dates in English:",
        "• 123 one hundred twenty-three",
        "• 456 four hundred fifty-six",
        "• March 15, 2024",
        "• December 31, 2023",
        "",
        "Common phrases:",
        "Please, thank you, you're welcome.",
        "How are you? I'm fine, thank you.",
        "Good morning, good afternoon, good evening.",
        "",
        "The OCR system should correctly recognize",
        "all this English content, including proper",
        "capitalization and punctuation marks.",
        "",
        "Technical terms and abbreviations:",
        "API, REST, JSON, XML, HTTP, HTTPS",
        "CPU, RAM, SSD, USB, WiFi, Bluetooth",
    ]
    
    for line in english_lines:
        if line:
            c.drawString(72, y_position, line)
        y_position -= 18
        if y_position < 50:
            c.showPage()
            y_position = height - 50
    
    c.save()
    print(f"Created: {pdf_path}")
    
    # Mixed language PDF
    pdf_path = f"{test_dir}/mixed_language_test.pdf"
    c = canvas.Canvas(pdf_path, pagesize=letter)
    
    c.setFont("Helvetica", 14)
    y_position = height - 80
    
    # Title
    c.drawString(72, y_position, "Documento Bilingüe / Bilingual Document")
    y_position -= 40
    
    c.setFont("Helvetica", 12)
    mixed_lines = [
        "Sección en español:",
        "",
        "Este es un documento que contiene texto en dos",
        "idiomas diferentes. El reconocimiento óptico",
        "de caracteres debe manejar ambos idiomas",
        "correctamente y sin confusión.",
        "",
        "Palabras clave: español, idioma, reconocimiento",
        "",
        "English section:",
        "",
        "This is a document that contains text in two", 
        "different languages. The optical character",
        "recognition should handle both languages",
        "correctly without confusion.",
        "",
        "Keywords: English, language, recognition",
        "",
        "Conclusión / Conclusion:",
        "",
        "Los sistemas modernos de OCR deben ser capaces",
        "de procesar múltiples idiomas en un solo documento.",
        "",
        "Modern OCR systems should be capable of processing",
        "multiple languages within a single document.",
    ]
    
    for line in mixed_lines:
        if line:
            c.drawString(72, y_position, line)
        y_position -= 18
        if y_position < 50:
            c.showPage()
            y_position = height - 50
    
    c.save()
    print(f"Created: {pdf_path}")
    
    # Complex Spanish document with special characters
    pdf_path = f"{test_dir}/spanish_complex.pdf"
    c = canvas.Canvas(pdf_path, pagesize=letter)
    
    c.setFont("Helvetica", 14)
    y_position = height - 80
    
    c.drawString(72, y_position, "Documento Español Complejo")
    y_position -= 40
    
    c.setFont("Helvetica", 12)
    complex_spanish_lines = [
        "Características especiales del español:",
        "",
        "Vocales acentuadas: á, é, í, ó, ú",
        "Letra eñe: niño, España, año, señor",
        "Diéresis: pingüino, cigüeña, vergüenza",
        "",
        "Signos de puntuación especiales:",
        "¿Preguntas con signos de apertura?",
        "¡Exclamaciones con signos de apertura!",
        "",
        "Palabras con combinaciones complejas:",
        "• excelente, exacto, oxígeno",
        "• desarrollo, rápido, árbol",
        "• comunicación, administración, información",
        "",
        "Números ordinales:",
        "1º primero, 2º segundo, 3º tercero",
        "10º décimo, 20º vigésimo, 100º centésimo",
        "",
        "Este documento prueba la capacidad del OCR",
        "para reconocer correctamente todos los",
        "caracteres especiales del idioma español.",
    ]
    
    for line in complex_spanish_lines:
        if line:
            c.drawString(72, y_position, line)
        y_position -= 18
        if y_position < 50:
            c.showPage()
            y_position = height - 50
    
    c.save()
    print(f"Created: {pdf_path}")
    
    # Complex English document
    pdf_path = f"{test_dir}/english_complex.pdf"
    c = canvas.Canvas(pdf_path, pagesize=letter)
    
    c.setFont("Helvetica", 14)
    y_position = height - 80
    
    c.drawString(72, y_position, "Complex English Document")
    y_position -= 40
    
    c.setFont("Helvetica", 12)
    complex_english_lines = [
        "Advanced English language features:",
        "",
        "Contractions: don't, won't, can't, isn't",
        "Possessives: user's, system's, company's",
        "Hyphenated words: state-of-the-art, well-known",
        "",
        "Technical terminology:",
        "• machine learning, artificial intelligence",
        "• natural language processing, deep learning",
        "• computer vision, pattern recognition",
        "",
        "Abbreviations and acronyms:",
        "• CEO, CTO, API, SDK, IDE, URL",
        "• HTML, CSS, JavaScript, TypeScript",
        "• REST, GraphQL, JSON, XML, YAML",
        "",
        "Numbers and measurements:",
        "• 3.14159 (pi), 2.71828 (e)",
        "• 100%, 50°F, 25°C, $1,000.00",
        "• 1st, 2nd, 3rd, 21st century",
        "",
        "This document tests the OCR system's ability",
        "to recognize complex English text patterns",
        "including technical terms and formatting.",
    ]
    
    for line in complex_english_lines:
        if line:
            c.drawString(72, y_position, line)
        y_position -= 18
        if y_position < 50:
            c.showPage()
            y_position = height - 50
    
    c.save()
    print(f"Created: {pdf_path}")
    
    print("\n🌍 Multilingual Test Files Summary:")
    print("=" * 50)
    
    # Check file sizes
    test_files = [
        "spanish_test.pdf",
        "english_test.pdf",
        "mixed_language_test.pdf",
        "spanish_complex.pdf",
        "english_complex.pdf"
    ]
    
    for filename in test_files:
        filepath = f"{test_dir}/{filename}"
        if os.path.exists(filepath):
            size_bytes = os.path.getsize(filepath)
            size_kb = size_bytes / 1024
            print(f"📄 {filename}: {size_kb:.1f} KB ({size_bytes:,} bytes)")
    
    print(f"\n✅ All multilingual test PDFs created in: {test_dir}/")
    print("🔤 Languages: Spanish (spa) and English (eng)")
    print("📝 Ready for OCR multiple language testing!")
    return True

if __name__ == "__main__":
    create_multilingual_test_pdfs()