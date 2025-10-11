use readur::config::Config;
use readur::oidc::OidcClient;
use wiremock::{matchers::{method, path}, Mock, MockServer, ResponseTemplate};

fn create_test_config_with_oidc(issuer_url: &str) -> Config {
    Config {
        database_url: "postgresql://test:test@localhost/test".to_string(),
        server_address: "127.0.0.1:8000".to_string(),
        jwt_secret: "test-secret".to_string(),
        upload_path: "./test-uploads".to_string(),
        watch_folder: "./test-watch".to_string(),
        user_watch_base_dir: "./user_watch".to_string(),
        enable_per_user_watch: false,
        allowed_file_types: vec!["pdf".to_string()],
        watch_interval_seconds: Some(30),
        file_stability_check_ms: Some(500),
        max_file_age_hours: None,
        ocr_language: "eng".to_string(),
        concurrent_ocr_jobs: 2,
        ocr_timeout_seconds: 60,
        max_file_size_mb: 10,
        memory_limit_mb: 256,
        cpu_priority: "normal".to_string(),
        oidc_enabled: true,
        oidc_client_id: Some("test-client-id".to_string()),
        oidc_client_secret: Some("test-client-secret".to_string()),
        oidc_issuer_url: Some(issuer_url.to_string()),
        oidc_redirect_uri: Some("http://localhost:8000/auth/oidc/callback".to_string()),
        oidc_auto_register: true,
        allow_local_auth: true,
        s3_enabled: false,
        s3_config: None,
    }
}

#[tokio::test]
async fn test_oidc_discovery() {
    let mock_server = MockServer::start().await;
    
    let discovery_response = serde_json::json!({
        "issuer": mock_server.uri(),
        "authorization_endpoint": format!("{}/auth", mock_server.uri()),
        "token_endpoint": format!("{}/token", mock_server.uri()),
        "userinfo_endpoint": format!("{}/userinfo", mock_server.uri())
    });

    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(discovery_response))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_oidc(&mock_server.uri());
    let oidc_client = OidcClient::new(&config).await;

    assert!(oidc_client.is_ok());
    let client = oidc_client.unwrap();
    assert_eq!(client.get_discovery().issuer, mock_server.uri());
    assert_eq!(client.get_discovery().authorization_endpoint, format!("{}/auth", mock_server.uri()));
}

#[tokio::test]
async fn test_oidc_discovery_failure() {
    let mock_server = MockServer::start().await;
    
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_oidc(&mock_server.uri());
    let oidc_client = OidcClient::new(&config).await;

    assert!(oidc_client.is_err());
}

#[tokio::test]
async fn test_get_authorization_url() {
    let mock_server = MockServer::start().await;
    
    let discovery_response = serde_json::json!({
        "issuer": mock_server.uri(),
        "authorization_endpoint": format!("{}/auth", mock_server.uri()),
        "token_endpoint": format!("{}/token", mock_server.uri()),
        "userinfo_endpoint": format!("{}/userinfo", mock_server.uri())
    });

    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(discovery_response))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_oidc(&mock_server.uri());
    let oidc_client = OidcClient::new(&config).await.unwrap();
    
    let (auth_url, csrf_token) = oidc_client.get_authorization_url();
    
    assert!(auth_url.to_string().contains("/auth"));
    assert!(auth_url.to_string().contains("client_id=test-client-id"));
    assert!(auth_url.to_string().contains("scope=openid+email+profile"));
    assert!(!csrf_token.secret().is_empty());
}

#[tokio::test]
async fn test_get_user_info() {
    let mock_server = MockServer::start().await;
    
    let discovery_response = serde_json::json!({
        "issuer": mock_server.uri(),
        "authorization_endpoint": format!("{}/auth", mock_server.uri()),
        "token_endpoint": format!("{}/token", mock_server.uri()),
        "userinfo_endpoint": format!("{}/userinfo", mock_server.uri())
    });

    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(discovery_response))
        .mount(&mock_server)
        .await;

    let user_info_response = serde_json::json!({
        "sub": "test-user-123",
        "email": "test@example.com",
        "name": "Test User",
        "preferred_username": "testuser"
    });

    Mock::given(method("GET"))
        .and(path("/userinfo"))
        .respond_with(ResponseTemplate::new(200).set_body_json(user_info_response))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_oidc(&mock_server.uri());
    let oidc_client = OidcClient::new(&config).await.unwrap();
    
    let user_info = oidc_client.get_user_info("test-access-token").await;
    
    assert!(user_info.is_ok());
    let info = user_info.unwrap();
    assert_eq!(info.sub, "test-user-123");
    assert_eq!(info.email, Some("test@example.com".to_string()));
    assert_eq!(info.preferred_username, Some("testuser".to_string()));
}

#[tokio::test]
async fn test_get_user_info_unauthorized() {
    let mock_server = MockServer::start().await;
    
    let discovery_response = serde_json::json!({
        "issuer": mock_server.uri(),
        "authorization_endpoint": format!("{}/auth", mock_server.uri()),
        "token_endpoint": format!("{}/token", mock_server.uri()),
        "userinfo_endpoint": format!("{}/userinfo", mock_server.uri())
    });

    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(discovery_response))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/userinfo"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let config = create_test_config_with_oidc(&mock_server.uri());
    let oidc_client = OidcClient::new(&config).await.unwrap();
    
    let user_info = oidc_client.get_user_info("invalid-access-token").await;
    
    assert!(user_info.is_err());
}

#[test]
fn test_oidc_config_validation() {
    let mut config = create_test_config_with_oidc("https://test.example.com");
    
    // Test missing client ID
    config.oidc_client_id = None;
    assert!(tokio_test::block_on(OidcClient::new(&config)).is_err());
    
    // Test missing client secret
    config.oidc_client_id = Some("test-client-id".to_string());
    config.oidc_client_secret = None;
    assert!(tokio_test::block_on(OidcClient::new(&config)).is_err());
    
    // Test missing issuer URL
    config.oidc_client_secret = Some("test-client-secret".to_string());
    config.oidc_issuer_url = None;
    assert!(tokio_test::block_on(OidcClient::new(&config)).is_err());
    
    // Test missing redirect URI
    config.oidc_issuer_url = Some("https://test.example.com".to_string());
    config.oidc_redirect_uri = None;
    assert!(tokio_test::block_on(OidcClient::new(&config)).is_err());
}