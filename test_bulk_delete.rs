// Simple test file to verify BulkDeleteRequest compilation
use readur::routes::documents::BulkDeleteRequest;
use uuid::Uuid;

fn main() {
    // Create a BulkDeleteRequest to test compilation
    let request = BulkDeleteRequest {
        document_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
    };
    
    // Test serialization
    let json = serde_json::to_string(&request).unwrap();
    println!("JSON: {}", json);
    
    // Test deserialization
    let deserialized: BulkDeleteRequest = serde_json::from_str(&json).unwrap();
    println!("Deserialized IDs count: {}", deserialized.document_ids.len());
    
    println!("BulkDeleteRequest compilation test passed!");
}