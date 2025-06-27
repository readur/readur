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
    pub fn get_discovery(&self) -> &OidcDiscovery {
        &self.discovery
    }

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

