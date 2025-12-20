use anyhow::{anyhow, Result};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId,
    ClientSecret, CsrfToken, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
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

// Storage for PKCE verifiers (csrf_token -> (verifier, expiry))
type PkceStore = Mutex<HashMap<String, (PkceCodeVerifier, Instant)>>;

#[derive(Debug)]
pub struct OidcClient {
    // Store configuration instead of the oauth2 client to avoid complex typestate types.
    // The oauth2 5.0 crate uses a typestate pattern that makes storing a configured client
    // in a struct unwieldy. Instead, we store the config and build the client on-demand.
    // This is cheap: just struct construction with pre-validated URLs (no network calls).
    // The expensive OIDC discovery happens once in `new()`.
    client_id: String,
    client_secret: Option<String>,
    redirect_uri: String,
    discovery: OidcDiscovery,
    http_client: Client,
    is_public_client: bool,
    pkce_store: PkceStore,
}

impl OidcClient {
    pub fn get_discovery(&self) -> &OidcDiscovery {
        &self.discovery
    }

    pub async fn new(config: &Config) -> Result<Self> {
        let client_id = config
            .oidc_client_id
            .as_ref()
            .ok_or_else(|| anyhow!("OIDC client ID not configured"))?
            .clone();

        // Client secret is optional - if not provided, this is a public client
        let client_secret = config.oidc_client_secret.clone();
        let is_public_client = client_secret.is_none();

        let issuer_url = config
            .oidc_issuer_url
            .as_ref()
            .ok_or_else(|| anyhow!("OIDC issuer URL not configured"))?;
        let redirect_uri = config
            .oidc_redirect_uri
            .as_ref()
            .ok_or_else(|| anyhow!("OIDC redirect URI not configured"))?
            .clone();

        let http_client = Client::new();

        // Discover OIDC endpoints
        let discovery = Self::discover_endpoints(&http_client, issuer_url).await?;

        Ok(Self {
            client_id,
            client_secret,
            redirect_uri,
            discovery,
            http_client,
            is_public_client,
            pkce_store: Mutex::new(HashMap::new()),
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

    pub fn get_authorization_url(&self) -> Result<(Url, CsrfToken)> {
        // Clean up expired PKCE verifiers (older than 10 minutes)
        self.cleanup_expired_verifiers();

        // Build OAuth2 client on-demand - this is cheap (just struct construction, no I/O)
        let mut oauth_client = BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_auth_uri(AuthUrl::new(self.discovery.authorization_endpoint.clone())?)
            .set_token_uri(TokenUrl::new(self.discovery.token_endpoint.clone())?)
            .set_redirect_uri(RedirectUrl::new(self.redirect_uri.clone())?);

        if let Some(secret) = &self.client_secret {
            oauth_client = oauth_client.set_client_secret(ClientSecret::new(secret.clone()));
        }

        let mut auth_request = oauth_client
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()));

        // For public clients (no client_secret), PKCE is required for security
        // For confidential clients, PKCE is optional but we don't use it to avoid state management
        if self.is_public_client {
            let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
            auth_request = auth_request.set_pkce_challenge(pkce_challenge);

            // Store the verifier for later use in token exchange
            let (url, csrf_token) = auth_request.url();
            let mut store = self.pkce_store.lock().unwrap();
            store.insert(
                csrf_token.secret().clone(),
                (pkce_verifier, Instant::now() + Duration::from_secs(600)), // 10 minute expiry
            );
            Ok((url, csrf_token))
        } else {
            // Confidential client - no PKCE needed
            Ok(auth_request.url())
        }
    }

    fn cleanup_expired_verifiers(&self) {
        let mut store = self.pkce_store.lock().unwrap();
        let now = Instant::now();
        store.retain(|_, (_, expiry)| *expiry > now);
    }

    pub async fn exchange_code(&self, code: &str, state: Option<&str>) -> Result<String> {
        // Build OAuth2 client on-demand - this is cheap (just struct construction, no I/O)
        let mut oauth_client = BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_auth_uri(AuthUrl::new(self.discovery.authorization_endpoint.clone())?)
            .set_token_uri(TokenUrl::new(self.discovery.token_endpoint.clone())?)
            .set_redirect_uri(RedirectUrl::new(self.redirect_uri.clone())?);

        if let Some(secret) = &self.client_secret {
            oauth_client = oauth_client.set_client_secret(ClientSecret::new(secret.clone()));
        }

        let mut token_request = oauth_client
            .exchange_code(AuthorizationCode::new(code.to_string()));

        // For public clients, retrieve and use the PKCE verifier
        if self.is_public_client {
            if let Some(state_token) = state {
                let mut store = self.pkce_store.lock().unwrap();
                if let Some((verifier, _)) = store.remove(state_token) {
                    token_request = token_request.set_pkce_verifier(verifier);
                } else {
                    return Err(anyhow!("PKCE verifier not found for state token (expired or invalid)"));
                }
            } else {
                return Err(anyhow!("State parameter required for public client PKCE flow"));
            }
        }

        // Create HTTP client for token exchange with redirect disabled for SSRF protection
        let oauth_http_client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| anyhow!("Failed to build HTTP client: {}", e))?;

        let token_result = token_request
            .request_async(&oauth_http_client)
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
