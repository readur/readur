use std::env;
use std::process;
use tokio;
use anyhow::Result;

async fn test_pdftotext(file_path: &str) -> Result<(String, usize)> {
    println!("=== Testing pdftotext ===");
    
    let temp_text_path = format!("/tmp/debug_pdftotext_{}.txt", std::process::id());
    
    let output = tokio::process::Command::new("pdftotext")
        .arg("-layout")
        .arg(file_path)
        .arg(&temp_text_path)
        .output()
        .await?;
    
    println!("pdftotext exit status: {}", output.status);
    if !output.stderr.is_empty() {
        println!("pdftotext stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    if output.status.success() {
        if let Ok(text) = tokio::fs::read_to_string(&temp_text_path).await {
            let word_count = text.split_whitespace().count();
            println!("pdftotext extracted {} words", word_count);
            println!("First 200 chars: {:?}", &text.chars().take(200).collect::<String>());
            
            // Clean up
            let _ = tokio::fs::remove_file(&temp_text_path).await;
            return Ok((text, word_count));
        } else {
            println!("Failed to read pdftotext output file");
        }
    } else {
        println!("pdftotext failed");
    }
    
    Ok((String::new(), 0))
}

async fn test_ocrmypdf_sidecar(file_path: &str) -> Result<(String, usize)> {
    println!("\n=== Testing ocrmypdf --sidecar ===");
    
    let temp_text_path = format!("/tmp/debug_ocrmypdf_{}.txt", std::process::id());
    
    let output = tokio::process::Command::new("ocrmypdf")
        .arg("--sidecar")
        .arg(&temp_text_path)
        .arg(file_path)
        .arg("-")  // Dummy output
        .output()
        .await?;
    
    println!("ocrmypdf --sidecar exit status: {}", output.status);
    if !output.stderr.is_empty() {
        println!("ocrmypdf --sidecar stderr: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    if output.status.success() {
        if let Ok(text) = tokio::fs::read_to_string(&temp_text_path).await {
            let word_count = text.split_whitespace().count();
            println!("ocrmypdf --sidecar extracted {} words", word_count);
            println!("First 200 chars: {:?}", &text.chars().take(200).collect::<String>());
            
            // Clean up
            let _ = tokio::fs::remove_file(&temp_text_path).await;
            return Ok((text, word_count));
        } else {
            println!("Failed to read ocrmypdf sidecar output file");
        }
    } else {
        println!("ocrmypdf --sidecar failed");
    }
    
    Ok((String::new(), 0))
}

async fn test_direct_extraction(file_path: &str) -> Result<(String, usize)> {
    println!("\n=== Testing direct text extraction ===");
    
    let bytes = tokio::fs::read(file_path).await?;
    println!("PDF file size: {} bytes", bytes.len());
    
    // Look for readable ASCII text in the PDF
    let mut ascii_text = String::new();
    let mut current_word = String::new();
    
    for &byte in &bytes {
        if byte >= 32 && byte <= 126 {  // Printable ASCII
            current_word.push(byte as char);
        } else {
            if current_word.len() > 3 {  // Only keep words longer than 3 characters
                ascii_text.push_str(&current_word);
                ascii_text.push(' ');
            }
            current_word.clear();
        }
    }
    
    // Add the last word if it's long enough
    if current_word.len() > 3 {
        ascii_text.push_str(&current_word);
    }
    
    // Clean up the text
    let cleaned_text = ascii_text
        .split_whitespace()
        .filter(|word| word.len() > 1)  // Filter out single characters
        .collect::<Vec<_>>()
        .join(" ");
    
    let word_count = cleaned_text.split_whitespace().count();
    println!("Direct extraction got {} words", word_count);
    println!("First 200 chars: {:?}", &cleaned_text.chars().take(200).collect::<String>());
    
    Ok((cleaned_text, word_count))
}

async fn test_quality_assessment(text: &str, word_count: usize, file_size: u64) {
    println!("\n=== Testing quality assessment ===");
    
    // Replicate the quality assessment logic
    if word_count == 0 {
        println!("Quality check: FAIL - no words");
        return;
    }
    
    // For very small files, low word count might be normal
    if file_size < 50_000 && word_count >= 1 {
        println!("Quality check: PASS - small file with some text");
        return;
    }
    
    // Calculate word density (words per KB)
    let file_size_kb = (file_size as f64) / 1024.0;
    let word_density = (word_count as f64) / file_size_kb;
    
    const MIN_WORD_DENSITY: f64 = 5.0;
    const MIN_WORDS_FOR_LARGE_FILES: usize = 10;
    const SUBSTANTIAL_WORD_COUNT: usize = 50;
    
    println!("File size: {:.1} KB", file_size_kb);
    println!("Word density: {:.2} words/KB", word_density);
    
    // If we have substantial text, accept it regardless of density
    if word_count >= SUBSTANTIAL_WORD_COUNT {
        println!("Quality check: PASS - substantial text content ({} words)", word_count);
        return;
    }
    
    if word_density < MIN_WORD_DENSITY && word_count < MIN_WORDS_FOR_LARGE_FILES {
        println!("Quality check: FAIL - appears to be image-based ({} words, {:.2} words/KB)", word_count, word_density);
        return;
    }
    
    // Additional check: if text is mostly non-alphanumeric, might be extraction artifacts
    let alphanumeric_chars = text.chars().filter(|c| c.is_alphanumeric()).count();
    let alphanumeric_ratio = if text.len() > 0 {
        (alphanumeric_chars as f64) / (text.len() as f64)
    } else {
        0.0
    };
    
    println!("Alphanumeric ratio: {:.1}%", alphanumeric_ratio * 100.0);
    
    // If less than 30% alphanumeric content, likely poor extraction
    if alphanumeric_ratio < 0.3 {
        println!("Quality check: FAIL - low alphanumeric content ({:.1}%)", alphanumeric_ratio * 100.0);
        return;
    }
    
    println!("Quality check: PASS - {} words, {:.2} words/KB, {:.1}% alphanumeric", 
             word_count, word_density, alphanumeric_ratio * 100.0);
}

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <pdf_file_path>", args[0]);
        process::exit(1);
    }
    
    let pdf_path = &args[1];
    println!("Debugging PDF extraction for: {}", pdf_path);
    
    // Check if file exists
    if !tokio::fs::metadata(pdf_path).await.is_ok() {
        eprintln!("Error: File '{}' not found", pdf_path);
        process::exit(1);
    }
    
    let file_size = tokio::fs::metadata(pdf_path).await?.len();
    println!("File size: {} bytes ({:.2} MB)", file_size, file_size as f64 / (1024.0 * 1024.0));
    
    // Test each extraction method
    let (pdftotext_text, pdftotext_words) = test_pdftotext(pdf_path).await?;
    let (ocrmypdf_text, ocrmypdf_words) = test_ocrmypdf_sidecar(pdf_path).await?;
    let (direct_text, direct_words) = test_direct_extraction(pdf_path).await?;
    
    // Test quality assessment on each result
    if pdftotext_words > 0 {
        test_quality_assessment(&pdftotext_text, pdftotext_words, file_size).await;
    }
    
    if ocrmypdf_words > 0 {
        test_quality_assessment(&ocrmypdf_text, ocrmypdf_words, file_size).await;
    }
    
    if direct_words > 0 {
        test_quality_assessment(&direct_text, direct_words, file_size).await;
    }
    
    println!("\n=== Summary ===");
    println!("pdftotext: {} words", pdftotext_words);
    println!("ocrmypdf --sidecar: {} words", ocrmypdf_words);
    println!("direct extraction: {} words", direct_words);
    
    // Determine what should happen based on the logic
    if pdftotext_words > 5 {
        println!("Expected result: Use pdftotext ({} words)", pdftotext_words);
    } else if direct_words > 5 {
        println!("Expected result: Use direct extraction ({} words)", direct_words);
    } else if ocrmypdf_words > 0 {
        println!("Expected result: Use ocrmypdf --sidecar ({} words)", ocrmypdf_words);
    } else {
        println!("Expected result: All methods failed");
    }
    
    Ok(())
}