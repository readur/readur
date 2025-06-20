use reqwest::{Client, Response};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};

#[derive(Debug)]
struct TestClient {
    client: Client,
    base_url: String,
    auth_token: Option<String>,
}

impl TestClient {
    fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "http://localhost:3001".to_string(),
            auth_token: None,
        }
    }

    async fn check_server_health(&self) -> Result<(), Box<dyn std::error::Error>> {
        let response = self
            .client
            .get(&format!("{}/api/health", self.base_url))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("Server health check failed: {}", response.status()).into())
        }
    }

    async fn register_user(&mut self, username: &str, email: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        let response = self
            .client
            .post(&format!("{}/api/auth/register", self.base_url))
            .json(&json!({
                "username": username,
                "email": email,
                "password": password
            }))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(format!("Registration failed: {}", error_text).into())
        }
    }

    async fn login(&mut self, username: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        let response = self
            .client
            .post(&format!("{}/api/auth/login", self.base_url))
            .json(&json!({
                "username": username,
                "password": password
            }))
            .send()
            .await?;

        if response.status().is_success() {
            let login_response: Value = response.json().await?;
            if let Some(token) = login_response["token"].as_str() {
                self.auth_token = Some(token.to_string());
                Ok(())
            } else {
                Err("No token in login response".into())
            }
        } else {
            let error_text = response.text().await?;
            Err(format!("Login failed: {}", error_text).into())
        }
    }

    fn get_auth_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        if let Some(token) = &self.auth_token {
            headers.insert("Authorization".to_string(), format!("Bearer {}", token));
        }
        headers
    }

    async fn create_label(&self, name: &str, description: Option<&str>, color: &str, icon: Option<&str>) -> Result<Value, Box<dyn std::error::Error>> {
        let mut payload = json!({
            "name": name,
            "color": color
        });

        if let Some(desc) = description {
            payload["description"] = json!(desc);
        }

        if let Some(icon_name) = icon {
            payload["icon"] = json!(icon_name);
        }

        let mut request = self
            .client
            .post(&format!("{}/api/labels", self.base_url))
            .json(&payload);

        for (key, value) in self.get_auth_headers() {
            request = request.header(&key, &value);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            let label: Value = response.json().await?;
            Ok(label)
        } else {
            let error_text = response.text().await?;
            Err(format!("Failed to create label: {}", error_text).into())
        }
    }

    async fn get_labels(&self, include_counts: bool) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let url = if include_counts {
            format!("{}/api/labels?include_counts=true", self.base_url)
        } else {
            format!("{}/api/labels", self.base_url)
        };

        let mut request = self.client.get(&url);

        for (key, value) in self.get_auth_headers() {
            request = request.header(&key, &value);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            let labels: Vec<Value> = response.json().await?;
            Ok(labels)
        } else {
            let error_text = response.text().await?;
            Err(format!("Failed to get labels: {}", error_text).into())
        }
    }

    async fn update_label(&self, label_id: &str, updates: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let mut request = self
            .client
            .put(&format!("{}/api/labels/{}", self.base_url, label_id))
            .json(&updates);

        for (key, value) in self.get_auth_headers() {
            request = request.header(&key, &value);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            let label: Value = response.json().await?;
            Ok(label)
        } else {
            let error_text = response.text().await?;
            Err(format!("Failed to update label: {}", error_text).into())
        }
    }

    async fn delete_label(&self, label_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut request = self
            .client
            .delete(&format!("{}/api/labels/{}", self.base_url, label_id));

        for (key, value) in self.get_auth_headers() {
            request = request.header(&key, &value);
        }

        let response = request.send().await?;

        if response.status().is_success() || response.status().as_u16() == 204 {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(format!("Failed to delete label: {}", error_text).into())
        }
    }

    async fn upload_document(&self, content: &[u8], filename: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let form = reqwest::multipart::Form::new()
            .part("file", reqwest::multipart::Part::bytes(content.to_vec())
                .file_name(filename.to_string())
                .mime_str("text/plain")?);

        let mut request = self
            .client
            .post(&format!("{}/api/documents", self.base_url))
            .multipart(form);

        for (key, value) in self.get_auth_headers() {
            request = request.header(&key, &value);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            let document: Value = response.json().await?;
            Ok(document)
        } else {
            let error_text = response.text().await?;
            Err(format!("Failed to upload document: {}", error_text).into())
        }
    }

    async fn assign_label_to_document(&self, document_id: &str, label_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut request = self
            .client
            .post(&format!("{}/api/labels/documents/{}/labels/{}", self.base_url, document_id, label_id));

        for (key, value) in self.get_auth_headers() {
            request = request.header(&key, &value);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(format!("Failed to assign label to document: {}", error_text).into())
        }
    }

    async fn get_document_labels(&self, document_id: &str) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let mut request = self
            .client
            .get(&format!("{}/api/labels/documents/{}", self.base_url, document_id));

        for (key, value) in self.get_auth_headers() {
            request = request.header(&key, &value);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            let labels: Vec<Value> = response.json().await?;
            Ok(labels)
        } else {
            let error_text = response.text().await?;
            Err(format!("Failed to get document labels: {}", error_text).into())
        }
    }

    async fn update_document_labels(&self, document_id: &str, label_ids: Vec<&str>) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let payload = json!({
            "label_ids": label_ids
        });

        let mut request = self
            .client
            .put(&format!("{}/api/labels/documents/{}", self.base_url, document_id))
            .json(&payload);

        for (key, value) in self.get_auth_headers() {
            request = request.header(&key, &value);
        }

        let response = request.send().await?;

        if response.status().is_success() {
            let labels: Vec<Value> = response.json().await?;
            Ok(labels)
        } else {
            let error_text = response.text().await?;
            Err(format!("Failed to update document labels: {}", error_text).into())
        }
    }

    async fn remove_label_from_document(&self, document_id: &str, label_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut request = self
            .client
            .delete(&format!("{}/api/labels/documents/{}/labels/{}", self.base_url, document_id, label_id));

        for (key, value) in self.get_auth_headers() {
            request = request.header(&key, &value);
        }

        let response = request.send().await?;

        if response.status().is_success() || response.status().as_u16() == 204 {
            Ok(())
        } else {
            let error_text = response.text().await?;
            Err(format!("Failed to remove label from document: {}", error_text).into())
        }
    }
}

#[tokio::test]
async fn test_label_crud_operations() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = TestClient::new();
    
    // Check server health
    client.check_server_health().await?;

    // Create unique user
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let username = format!("label_test_user_{}", timestamp);
    let email = format!("label_test_{}@example.com", timestamp);
    
    client.register_user(&username, &email, "test_password").await?;
    client.login(&username, "test_password").await?;

    // Test: Create label
    println!("Testing label creation...");
    let created_label = client.create_label(
        "Test Label",
        Some("A test label for integration testing"),
        "#ff0000",
        Some("star")
    ).await?;

    assert_eq!(created_label["name"], "Test Label");
    assert_eq!(created_label["color"], "#ff0000");
    assert_eq!(created_label["icon"], "star");
    assert_eq!(created_label["is_system"], false);

    let label_id = created_label["id"].as_str().unwrap();

    // Test: Get all labels
    println!("Testing label retrieval...");
    let labels = client.get_labels(false).await?;
    assert!(labels.len() >= 1);
    
    let found_label = labels.iter().find(|l| l["id"] == label_id);
    assert!(found_label.is_some());

    // Test: Get labels with counts
    let labels_with_counts = client.get_labels(true).await?;
    assert!(labels_with_counts.len() >= 1);
    
    let found_label_with_count = labels_with_counts.iter().find(|l| l["id"] == label_id);
    assert!(found_label_with_count.is_some());
    assert_eq!(found_label_with_count.unwrap()["document_count"], 0);

    // Test: Update label
    println!("Testing label update...");
    let updates = json!({
        "name": "Updated Test Label",
        "color": "#00ff00",
        "description": "Updated description"
    });

    let updated_label = client.update_label(label_id, updates).await?;
    assert_eq!(updated_label["name"], "Updated Test Label");
    assert_eq!(updated_label["color"], "#00ff00");
    assert_eq!(updated_label["description"], "Updated description");

    // Test: Delete label
    println!("Testing label deletion...");
    client.delete_label(label_id).await?;

    // Verify deletion
    let labels_after_delete = client.get_labels(false).await?;
    let deleted_label = labels_after_delete.iter().find(|l| l["id"] == label_id);
    assert!(deleted_label.is_none());

    println!("Label CRUD operations test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_document_label_assignment() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = TestClient::new();
    
    // Check server health
    client.check_server_health().await?;

    // Create unique user
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let username = format!("doc_label_test_user_{}", timestamp);
    let email = format!("doc_label_test_{}@example.com", timestamp);
    
    client.register_user(&username, &email, "test_password").await?;
    client.login(&username, "test_password").await?;

    // Create labels
    println!("Creating test labels...");
    let label1 = client.create_label("Important", Some("High priority items"), "#ff0000", Some("star")).await?;
    let label2 = client.create_label("Work", Some("Work-related documents"), "#0000ff", Some("work")).await?;
    let label3 = client.create_label("Personal", Some("Personal documents"), "#00ff00", Some("person")).await?;

    let label1_id = label1["id"].as_str().unwrap();
    let label2_id = label2["id"].as_str().unwrap();
    let label3_id = label3["id"].as_str().unwrap();

    // Upload a test document
    println!("Uploading test document...");
    let document_content = b"This is a test document for label assignment testing.";
    let document = client.upload_document(document_content, "test_document.txt").await?;
    let document_id = document["id"].as_str().unwrap();

    // Test: Assign single label
    println!("Testing single label assignment...");
    client.assign_label_to_document(document_id, label1_id).await?;

    let document_labels = client.get_document_labels(document_id).await?;
    assert_eq!(document_labels.len(), 1);
    assert_eq!(document_labels[0]["id"], label1_id);

    // Test: Assign additional label
    println!("Testing additional label assignment...");
    client.assign_label_to_document(document_id, label2_id).await?;

    let document_labels = client.get_document_labels(document_id).await?;
    assert_eq!(document_labels.len(), 2);

    // Test: Update document labels (replace)
    println!("Testing label replacement...");
    let new_labels = client.update_document_labels(document_id, vec![label2_id, label3_id]).await?;
    assert_eq!(new_labels.len(), 2);
    
    let label_ids: Vec<&str> = new_labels.iter()
        .map(|l| l["id"].as_str().unwrap())
        .collect();
    assert!(label_ids.contains(&label2_id));
    assert!(label_ids.contains(&label3_id));
    assert!(!label_ids.contains(&label1_id));

    // Test: Remove single label
    println!("Testing single label removal...");
    client.remove_label_from_document(document_id, label2_id).await?;

    let document_labels = client.get_document_labels(document_id).await?;
    assert_eq!(document_labels.len(), 1);
    assert_eq!(document_labels[0]["id"], label3_id);

    // Test: Remove all labels
    println!("Testing removal of all labels...");
    client.update_document_labels(document_id, vec![]).await?;

    let document_labels = client.get_document_labels(document_id).await?;
    assert_eq!(document_labels.len(), 0);

    // Test: Verify label usage counts
    println!("Testing label usage counts...");
    let labels_with_counts = client.get_labels(true).await?;
    
    for label in labels_with_counts {
        let label_id = label["id"].as_str().unwrap();
        if label_id == label1_id || label_id == label2_id || label_id == label3_id {
            assert_eq!(label["document_count"], 0);
        }
    }

    println!("Document label assignment test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_system_labels_access() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = TestClient::new();
    
    // Check server health
    client.check_server_health().await?;

    // Create unique user
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let username = format!("system_label_test_user_{}", timestamp);
    let email = format!("system_label_test_{}@example.com", timestamp);
    
    client.register_user(&username, &email, "test_password").await?;
    client.login(&username, "test_password").await?;

    // Test: Get labels (should include system labels)
    println!("Testing system labels access...");
    let labels = client.get_labels(false).await?;
    
    // Should have system labels
    let system_labels: Vec<&Value> = labels.iter()
        .filter(|l| l["is_system"].as_bool().unwrap_or(false))
        .collect();
    
    assert!(system_labels.len() > 0, "Should have system labels");

    // Check for some expected system labels
    let system_label_names: Vec<&str> = system_labels.iter()
        .map(|l| l["name"].as_str().unwrap())
        .collect();

    let expected_system_labels = vec!["Important", "Archive", "Personal", "Work"];
    for expected in expected_system_labels {
        assert!(
            system_label_names.contains(&expected),
            "Expected system label '{}' not found. Available: {:?}",
            expected,
            system_label_names
        );
    }

    // Test: Upload document and assign system label
    println!("Testing system label assignment...");
    let document_content = b"This is a test document for system label assignment.";
    let document = client.upload_document(document_content, "system_label_test.txt").await?;
    let document_id = document["id"].as_str().unwrap();

    // Find a system label to assign
    let important_label = system_labels.iter()
        .find(|l| l["name"] == "Important")
        .expect("Important system label not found");
    
    let important_label_id = important_label["id"].as_str().unwrap();

    // Assign system label to document
    client.assign_label_to_document(document_id, important_label_id).await?;

    let document_labels = client.get_document_labels(document_id).await?;
    assert_eq!(document_labels.len(), 1);
    assert_eq!(document_labels[0]["id"], important_label_id);
    assert_eq!(document_labels[0]["is_system"], true);

    println!("System labels access test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_label_validation() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = TestClient::new();
    
    // Check server health
    client.check_server_health().await?;

    // Create unique user
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let username = format!("validation_test_user_{}", timestamp);
    let email = format!("validation_test_{}@example.com", timestamp);
    
    client.register_user(&username, &email, "test_password").await?;
    client.login(&username, "test_password").await?;

    // Test: Invalid color format should fail
    println!("Testing invalid color validation...");
    let response = client.client
        .post(&format!("{}/api/labels", client.base_url))
        .header("Authorization", format!("Bearer {}", client.auth_token.as_ref().unwrap()))
        .json(&json!({
            "name": "Invalid Color",
            "color": "invalid_color"
        }))
        .send()
        .await?;

    assert!(!response.status().is_success(), "Should reject invalid color format");

    // Test: Empty name should fail
    println!("Testing empty name validation...");
    let response = client.client
        .post(&format!("{}/api/labels", client.base_url))
        .header("Authorization", format!("Bearer {}", client.auth_token.as_ref().unwrap()))
        .json(&json!({
            "name": "",
            "color": "#ff0000"
        }))
        .send()
        .await?;

    assert!(!response.status().is_success(), "Should reject empty name");

    // Test: Duplicate name should fail
    println!("Testing duplicate name validation...");
    let label_name = "Duplicate Test";
    
    // Create first label
    client.create_label(label_name, None, "#ff0000", None).await?;
    
    // Try to create duplicate
    let response = client.client
        .post(&format!("{}/api/labels", client.base_url))
        .header("Authorization", format!("Bearer {}", client.auth_token.as_ref().unwrap()))
        .json(&json!({
            "name": label_name,
            "color": "#00ff00"
        }))
        .send()
        .await?;

    assert!(!response.status().is_success(), "Should reject duplicate name");

    println!("Label validation test completed successfully!");
    Ok(())
}

#[tokio::test]
async fn test_label_permissions() -> Result<(), Box<dyn std::error::Error>> {
    let mut client1 = TestClient::new();
    let mut client2 = TestClient::new();
    
    // Check server health
    client1.check_server_health().await?;

    // Create two unique users
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    
    let username1 = format!("perm_test_user1_{}", timestamp);
    let email1 = format!("perm_test1_{}@example.com", timestamp);
    
    let username2 = format!("perm_test_user2_{}", timestamp);
    let email2 = format!("perm_test2_{}@example.com", timestamp);
    
    client1.register_user(&username1, &email1, "test_password").await?;
    client1.login(&username1, "test_password").await?;
    
    sleep(Duration::from_millis(100)).await; // Small delay to ensure unique timestamps
    
    client2.register_user(&username2, &email2, "test_password").await?;
    client2.login(&username2, "test_password").await?;

    // User 1 creates a label
    println!("Testing cross-user label access permissions...");
    let user1_label = client1.create_label("User 1 Label", None, "#ff0000", None).await?;
    let user1_label_id = user1_label["id"].as_str().unwrap();

    // User 2 should not be able to update User 1's label
    println!("Testing unauthorized label update...");
    let updates = json!({
        "name": "Hacked Label",
        "color": "#000000"
    });

    let response = client2.client
        .put(&format!("{}/api/labels/{}", client2.base_url, user1_label_id))
        .header("Authorization", format!("Bearer {}", client2.auth_token.as_ref().unwrap()))
        .json(&updates)
        .send()
        .await?;

    assert!(!response.status().is_success(), "User 2 should not be able to update User 1's label");

    // User 2 should not be able to delete User 1's label
    println!("Testing unauthorized label deletion...");
    let response = client2.client
        .delete(&format!("{}/api/labels/{}", client2.base_url, user1_label_id))
        .header("Authorization", format!("Bearer {}", client2.auth_token.as_ref().unwrap()))
        .send()
        .await?;

    assert!(!response.status().is_success(), "User 2 should not be able to delete User 1's label");

    // User 1's label should still exist and be unchanged
    println!("Verifying label integrity...");
    let user1_labels = client1.get_labels(false).await?;
    let found_label = user1_labels.iter().find(|l| l["id"] == user1_label_id);
    assert!(found_label.is_some());
    assert_eq!(found_label.unwrap()["name"], "User 1 Label");

    println!("Label permissions test completed successfully!");
    Ok(())
}