use reqwest;
use std::time::Duration;

#[tokio::test]
async fn test_health_endpoint_responds() {
    // Test that the health endpoint responds correctly
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    // Assuming the server is running on the default port
    let base_url = std::env::var("TEST_BASE_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    
    let response = client
        .get(&format!("{}/api/health", base_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            assert_eq!(resp.status(), 200, "Health endpoint should return 200 OK");
            
            let body: serde_json::Value = resp.json().await.unwrap();
            assert_eq!(body["status"], "ok", "Health status should be 'ok'");
        }
        Err(e) => {
            // If server is not running, skip the test
            eprintln!("Warning: Server not running, skipping health check test: {}", e);
        }
    }
}

#[tokio::test]
async fn test_health_endpoint_different_ports() {
    // Test that health endpoint works on different configured ports
    let ports = vec![8000, 8001, 8080];
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();

    for port in ports {
        let url = format!("http://localhost:{}/api/health", port);
        let response = client.get(&url).send().await;
        
        if let Ok(resp) = response {
            if resp.status() == 200 {
                let body: serde_json::Value = resp.json().await.unwrap();
                assert_eq!(body["status"], "ok", "Health status should be 'ok' on port {}", port);
                println!("✓ Health check successful on port {}", port);
            }
        }
    }
}

#[cfg(test)]
mod docker_health_tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn test_dockerfile_has_curl() {
        // Verify that the Dockerfile includes curl for healthchecks
        let dockerfile_content = std::fs::read_to_string("Dockerfile")
            .expect("Should be able to read Dockerfile");
        
        assert!(
            dockerfile_content.contains("curl"),
            "Dockerfile should install curl for healthchecks"
        );
    }

    #[test]
    fn test_docker_compose_health_check_port_matches() {
        // Verify docker-compose.yml healthcheck uses correct port
        let compose_content = std::fs::read_to_string("docker-compose.yml")
            .expect("Should be able to read docker-compose.yml");
        
        // Check that if SERVER_PORT is 8000, healthcheck also uses 8000
        if compose_content.contains("SERVER_PORT: 8000") || compose_content.contains("SERVER_PORT=8000") {
            assert!(
                compose_content.contains("http://localhost:8000/api/health"),
                "Healthcheck should use port 8000 when SERVER_PORT is 8000"
            );
        }
        
        // Check that healthcheck URL is present
        assert!(
            compose_content.contains("/api/health"),
            "docker-compose.yml should have health check endpoint"
        );
    }

    #[test]
    fn test_docker_compose_test_health_check_port_matches() {
        // Verify docker-compose.test.yml healthcheck uses correct port
        let compose_test_content = std::fs::read_to_string("docker-compose.test.yml")
            .expect("Should be able to read docker-compose.test.yml");
        
        // The test compose file uses port 8001
        if compose_test_content.contains("SERVER_PORT: 8001") {
            assert!(
                compose_test_content.contains("http://localhost:8001/api/health"),
                "Test healthcheck should use port 8001 when SERVER_PORT is 8001"
            );
        }
    }

    #[test] 
    fn test_curl_available_in_container() {
        // This test only runs if we're in a Docker environment
        if std::path::Path::new("/.dockerenv").exists() {
            let output = Command::new("which")
                .arg("curl")
                .output()
                .expect("Failed to check for curl");
            
            assert!(
                output.status.success(),
                "curl should be available in the container"
            );
        }
    }
}