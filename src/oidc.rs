use anyhow::{anyhow, Result};
use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, PkceCodeChallenge, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::config::Config;

#[derive(Debug, Serialize, Deserialize)]
pub struct OidcDiscovery {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub userinfo_endpoint: String,
    pub issuer: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OidcUserInfo {
    pub sub: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
}

#[derive(Debug)]
pub struct OidcClient {
    oauth_client: BasicClient,
    discovery: OidcDiscovery,
    http_client: Client,
}

impl OidcClient {
    pub async fn new(config: &Config) -> Result<Self> {
        let client_id = config
            .oidc_client_id
            .as_ref()
            .ok_or_else(|| anyhow!("OIDC client ID not configured"))?;
        let client_secret = config
            .oidc_client_secret
            .as_ref()
            .ok_or_else(|| anyhow!("OIDC client secret not configured"))?;
        let issuer_url = config
            .oidc_issuer_url
            .as_ref()
            .ok_or_else(|| anyhow!("OIDC issuer URL not configured"))?;
        let redirect_uri = config
            .oidc_redirect_uri
            .as_ref()
            .ok_or_else(|| anyhow!("OIDC redirect URI not configured"))?;

        let http_client = Client::new();

        // Discover OIDC endpoints
        let discovery = Self::discover_endpoints(&http_client, issuer_url).await?;

        // Create OAuth2 client
        let oauth_client = BasicClient::new(
            ClientId::new(client_id.clone()),
            Some(ClientSecret::new(client_secret.clone())),
            AuthUrl::new(discovery.authorization_endpoint.clone())?,
            Some(TokenUrl::new(discovery.token_endpoint.clone())?),
        )
        .set_redirect_uri(RedirectUrl::new(redirect_uri.clone())?);

        Ok(Self {
            oauth_client,
            discovery,
            http_client,
        })
    }

    async fn discover_endpoints(client: &Client, issuer_url: &str) -> Result<OidcDiscovery> {
        let discovery_url = format!("{}/.well-known/openid-configuration", issuer_url.trim_end_matches('/'));
        
        let response = client
            .get(&discovery_url)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch OIDC discovery document: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "OIDC discovery failed with status: {}",
                response.status()
            ));
        }

        let discovery: OidcDiscovery = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse OIDC discovery document: {}", e))?;

        Ok(discovery)
    }

    pub fn get_authorization_url(&self) -> (Url, CsrfToken) {
        let (pkce_challenge, _pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        self.oauth_client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url()
    }

    pub async fn exchange_code(&self, code: &str) -> Result<String> {
        let token_result = self
            .oauth_client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| anyhow!("Failed to exchange authorization code: {}", e))?;

        Ok(token_result.access_token().secret().clone())
    }

    pub async fn get_user_info(&self, access_token: &str) -> Result<OidcUserInfo> {
        let response = self
            .http_client
            .get(&self.discovery.userinfo_endpoint)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch user info: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "User info request failed with status: {}",
                response.status()
            ));
        }

        let user_info: OidcUserInfo = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse user info: {}", e))?;

        Ok(user_info)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OidcAuthResponse {
    pub user_id: uuid::Uuid,
    pub username: String,
    pub email: Option<String>,
    pub is_new_user: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use wiremock::{matchers::{method, path}, Mock, MockServer, ResponseTemplate};

    fn create_test_config_with_oidc(issuer_url: &str) -> Config {
        Config {
            database_url: "postgresql://test:test@localhost/test".to_string(),
            server_address: "127.0.0.1:8000".to_string(),
            jwt_secret: "test-secret".to_string(),
            upload_path: "./test-uploads".to_string(),
            watch_folder: "./test-watch".to_string(),
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
        assert_eq!(client.discovery.issuer, mock_server.uri());
        assert_eq!(client.discovery.authorization_endpoint, format!("{}/auth", mock_server.uri()));
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
}