use readur::config::Config;
use readur::oidc::OidcClient;
use readur::test_helpers::create_test_config_with_db;
use wiremock::{matchers::{method, path}, Mock, MockServer, ResponseTemplate};

fn create_test_config_with_oidc(issuer_url: &str) -> Config {
    let mut config = create_test_config_with_db("postgresql://test:test@localhost/test");
    config.server_address = "127.0.0.1:8000".to_string();
    config.jwt_secret = "test-secret".to_string();
    config.upload_path = "./test-uploads".to_string();
    config.watch_folder = "./test-watch".to_string();
    config.oidc_enabled = true;
    config.oidc_client_id = Some("test-client-id".to_string());
    config.oidc_client_secret = Some("test-client-secret".to_string());
    config.oidc_issuer_url = Some(issuer_url.to_string());
    config.oidc_redirect_uri = Some("http://localhost:8000/auth/oidc/callback".to_string());
    config.oidc_auto_register = Some(true);
    config.allow_local_auth = Some(true);
    config
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
    
    let start = oidc_client.begin_login().unwrap();
    let auth_url = start.authorization_url.clone();
    let params: std::collections::HashMap<String, String> =
        auth_url.query_pairs().into_owned().collect();

    assert_eq!(auth_url.path(), "/auth");
    assert_eq!(params.get("client_id").map(String::as_str), Some("test-client-id"));
    assert_eq!(params.get("scope").map(String::as_str), Some("openid email profile"));
    assert_eq!(params.get("response_type").map(String::as_str), Some("code"));
    assert_eq!(params.get("state"), Some(&start.state));
    assert_eq!(params.get("nonce"), Some(&start.pending.nonce));
    assert_eq!(params.get("code_challenge_method").map(String::as_str), Some("S256"));
    assert!(params.get("code_challenge").is_some_and(|c| !c.is_empty()));
    // The PKCE verifier is kept server-side and never sent in the URL.
    assert!(!auth_url.as_str().contains(&start.pending.pkce_verifier));

    // Each login gets fresh values.
    let second = oidc_client.begin_login().unwrap();
    assert_ne!(second.state, start.state);
    assert_ne!(second.pending.nonce, start.pending.nonce);
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
    
    // Test missing issuer URL (a missing client secret is allowed: public client)
    config.oidc_client_id = Some("test-client-id".to_string());
    config.oidc_issuer_url = None;
    assert!(tokio_test::block_on(OidcClient::new(&config)).is_err());
    
    // Test missing redirect URI
    config.oidc_issuer_url = Some("https://test.example.com".to_string());
    config.oidc_redirect_uri = None;
    assert!(tokio_test::block_on(OidcClient::new(&config)).is_err());
}