use std::fs;
use readur::metadata_extraction::extract_content_metadata;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing metadata extraction...");

    // Test image metadata
    if let Ok(image_data) = fs::read("test_files/portrait_100x200.png") {
        println!("\n=== Testing Image (portrait_100x200.png) ===");
        match extract_content_metadata(&image_data, "image/png", "portrait_100x200.png").await {
            Ok(Some(metadata)) => {
                println!("Metadata extracted:");
                println!("{:#}", serde_json::to_string_pretty(&metadata)?);
            }
            Ok(None) => println!("No metadata extracted"),
            Err(e) => println!("Error: {}", e),
        }
    }

    // Test PDF metadata
    if let Ok(pdf_data) = fs::read("test_files/single_page_v14.pdf") {
        println!("\n=== Testing PDF (single_page_v14.pdf) ===");
        match extract_content_metadata(&pdf_data, "application/pdf", "single_page_v14.pdf").await {
            Ok(Some(metadata)) => {
                println!("Metadata extracted:");
                println!("{:#}", serde_json::to_string_pretty(&metadata)?);
            }
            Ok(None) => println!("No metadata extracted"),
            Err(e) => println!("Error: {}", e),
        }
    }

    // Test text metadata
    if let Ok(text_data) = fs::read("test_files/comprehensive_text.txt") {
        println!("\n=== Testing Text (comprehensive_text.txt) ===");
        match extract_content_metadata(&text_data, "text/plain", "comprehensive_text.txt").await {
            Ok(Some(metadata)) => {
                println!("Metadata extracted:");
                println!("{:#}", serde_json::to_string_pretty(&metadata)?);
            }
            Ok(None) => println!("No metadata extracted"),
            Err(e) => println!("Error: {}", e),
        }
    }

    // Test JSON format detection
    if let Ok(json_data) = fs::read("test_files/test_format.json") {
        println!("\n=== Testing JSON Format (test_format.json) ===");
        match extract_content_metadata(&json_data, "text/plain", "test_format.json").await {
            Ok(Some(metadata)) => {
                println!("Metadata extracted:");
                println!("{:#}", serde_json::to_string_pretty(&metadata)?);
            }
            Ok(None) => println!("No metadata extracted"),
            Err(e) => println!("Error: {}", e),
        }
    }

    println!("\nMetadata extraction testing complete!");
    Ok(())
}