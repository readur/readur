//! OpenID Connect relying-party implementation (authorization code flow).
//!
//! Every login uses PKCE (S256), a random `state` and a random `nonce`, for
//! both public and confidential clients. The ID token is verified (signature,
//! issuer, audience, expiry, nonce) and is the source of the user's identity;
//! the userinfo endpoint only supplies optional profile fields.
//!
//! Persisting the per-login secrets (state -> PKCE verifier + nonce) is the
//! caller's job, so this module stays storage-agnostic.

use anyhow::{anyhow, Result};
use base64ct::Encoding;
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use rand::Rng;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use url::Url;

use crate::config::Config;

/// Minimum time between JWKS refreshes triggered by an unknown `kid`.
const JWKS_REFRESH_COOLDOWN: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcDiscovery {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    #[serde(default)]
    pub userinfo_endpoint: Option<String>,
    pub issuer: String,
    #[serde(default)]
    pub jwks_uri: Option<String>,
}

/// Profile information about the authenticated end-user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcUserInfo {
    pub sub: String,
    pub email: Option<String>,
    #[serde(default)]
    pub email_verified: Option<bool>,
    pub name: Option<String>,
    pub preferred_username: Option<String>,
}

/// Values that must be kept server-side between redirect and callback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingLogin {
    pub pkce_verifier: String,
    pub nonce: String,
}

/// Result of starting a login: where to send the browser, and what to store.
pub struct LoginStart {
    pub authorization_url: Url,
    pub state: String,
    pub pending: PendingLogin,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IdTokenClaims {
    sub: String,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    email_verified: Option<serde_json::Value>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    preferred_username: Option<String>,
}

pub struct OidcClient {
    client_id: String,
    client_secret: Option<String>,
    redirect_uri: String,
    discovery: OidcDiscovery,
    http_client: Client,
    jwks: RwLock<Option<(JwkSet, Instant)>>,
}

impl std::fmt::Debug for OidcClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OidcClient")
            .field("client_id", &self.client_id)
            .field("client_secret", &self.client_secret.as_ref().map(|_| "***"))
            .field("redirect_uri", &self.redirect_uri)
            .field("discovery", &self.discovery)
            .finish()
    }
}

/// Random URL-safe string carrying `bytes` bytes of entropy.
pub fn random_urlsafe(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::rng().fill(&mut buf[..]);
    base64ct::Base64UrlUnpadded::encode_string(&buf)
}

fn pkce_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64ct::Base64UrlUnpadded::encode_string(&digest)
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
        let client_secret = config.oidc_client_secret.clone().filter(|s| !s.is_empty());

        let issuer_url = config
            .oidc_issuer_url
            .as_ref()
            .ok_or_else(|| anyhow!("OIDC issuer URL not configured"))?;
        let redirect_uri = config
            .oidc_redirect_uri
            .as_ref()
            .ok_or_else(|| anyhow!("OIDC redirect URI not configured"))?
            .clone();
        Url::parse(&redirect_uri).map_err(|e| anyhow!("Invalid OIDC redirect URI: {}", e))?;

        // Never follow redirects on back-channel requests.
        let http_client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| anyhow!("Failed to build HTTP client: {}", e))?;

        let discovery = Self::discover_endpoints(&http_client, issuer_url).await?;

        Ok(Self {
            client_id,
            client_secret,
            redirect_uri,
            discovery,
            http_client,
            jwks: RwLock::new(None),
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

    /// Build the authorization request. The returned `state` must be bound to
    /// the browser (cookie) and `pending` stored server-side until callback.
    pub fn begin_login(&self) -> Result<LoginStart> {
        let state = random_urlsafe(32);
        let pending = PendingLogin {
            pkce_verifier: random_urlsafe(48),
            nonce: random_urlsafe(32),
        };

        let mut url = Url::parse(&self.discovery.authorization_endpoint)
            .map_err(|e| anyhow!("Invalid authorization endpoint: {}", e))?;
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", &self.redirect_uri)
            .append_pair("scope", "openid email profile")
            .append_pair("state", &state)
            .append_pair("nonce", &pending.nonce)
            .append_pair("code_challenge", &pkce_challenge(&pending.pkce_verifier))
            .append_pair("code_challenge_method", "S256");

        Ok(LoginStart {
            authorization_url: url,
            state,
            pending,
        })
    }

    /// Redeem an authorization code and return the verified identity.
    pub async fn complete_login(&self, code: &str, pending: &PendingLogin) -> Result<OidcUserInfo> {
        let tokens = self.exchange_code(code, &pending.pkce_verifier).await?;
        let id_token = tokens
            .id_token
            .as_deref()
            .ok_or_else(|| anyhow!("Token response did not include an id_token"))?;
        let claims = self.verify_id_token(id_token, &pending.nonce).await?;

        let mut info = OidcUserInfo {
            sub: claims.sub,
            email: claims.email,
            email_verified: claims.email_verified.as_ref().and_then(parse_bool_claim),
            name: claims.name,
            preferred_username: claims.preferred_username,
        };

        // Userinfo may carry profile claims absent from the ID token. Its
        // `sub` MUST match the ID token's (OIDC Core 5.3.2).
        if self.discovery.userinfo_endpoint.is_some() {
            match self.get_user_info(&tokens.access_token).await {
                Ok(extra) if extra.sub == info.sub => {
                    if info.email.is_none() {
                        info.email = extra.email;
                        info.email_verified = extra.email_verified;
                    }
                    info.name = info.name.or(extra.name);
                    info.preferred_username = info.preferred_username.or(extra.preferred_username);
                }
                Ok(_) => return Err(anyhow!("Userinfo subject does not match ID token subject")),
                Err(e) => tracing::warn!("OIDC userinfo lookup failed, using ID token claims only: {}", e),
            }
        }

        Ok(info)
    }

    async fn exchange_code(&self, code: &str, pkce_verifier: &str) -> Result<TokenResponse> {
        let mut form = vec![
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", self.redirect_uri.as_str()),
            ("code_verifier", pkce_verifier),
        ];

        let mut request = self.http_client.post(&self.discovery.token_endpoint);
        match &self.client_secret {
            Some(secret) => {
                request = request.basic_auth(
                    urlencoding::encode(&self.client_id),
                    Some(urlencoding::encode(secret)),
                );
            }
            None => form.push(("client_id", self.client_id.as_str())),
        }

        let response = request
            .header("accept", "application/json")
            .form(&form)
            .send()
            .await
            .map_err(|e| anyhow!("Token request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(anyhow!("Token endpoint returned status {}", response.status()));
        }

        response
            .json::<TokenResponse>()
            .await
            .map_err(|e| anyhow!("Failed to parse token response: {}", e))
    }

    async fn verify_id_token(&self, id_token: &str, expected_nonce: &str) -> Result<IdTokenClaims> {
        let header = decode_header(id_token).map_err(|e| anyhow!("Malformed ID token: {}", e))?;

        let key = match header.alg {
            Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
                // Symmetric ID tokens are keyed with the client secret.
                let secret = self
                    .client_secret
                    .as_ref()
                    .ok_or_else(|| anyhow!("HMAC-signed ID token but no client secret configured"))?;
                DecodingKey::from_secret(secret.as_bytes())
            }
            _ => self.signing_key(header.kid.as_deref()).await?,
        };

        let mut validation = Validation::new(header.alg);
        validation.leeway = 60;
        validation.set_issuer(&[self.discovery.issuer.as_str()]);
        validation.set_audience(&[self.client_id.as_str()]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);

        let claims = decode::<IdTokenClaims>(id_token, &key, &validation)
            .map_err(|e| anyhow!("ID token validation failed: {}", e))?
            .claims;

        let nonce_ok = claims
            .nonce
            .as_deref()
            .map(|n| crate::utils::security::constant_time_eq(n.as_bytes(), expected_nonce.as_bytes()))
            .unwrap_or(false);
        if !nonce_ok {
            return Err(anyhow!("ID token nonce mismatch"));
        }

        Ok(claims)
    }

    async fn signing_key(&self, kid: Option<&str>) -> Result<DecodingKey> {
        if let Some(key) = self.find_cached_key(kid).await {
            return Ok(key);
        }

        // Unknown key: refresh the JWKS (key rotation), rate-limited.
        let stale = match &*self.jwks.read().await {
            Some((_, fetched)) => fetched.elapsed() >= JWKS_REFRESH_COOLDOWN,
            None => true,
        };
        if stale {
            let jwks = self.fetch_jwks().await?;
            *self.jwks.write().await = Some((jwks, Instant::now()));
        }

        self.find_cached_key(kid)
            .await
            .ok_or_else(|| anyhow!("No matching signing key in provider JWKS"))
    }

    async fn find_cached_key(&self, kid: Option<&str>) -> Option<DecodingKey> {
        let guard = self.jwks.read().await;
        let (set, _) = guard.as_ref()?;
        let jwk = match kid {
            Some(kid) => set.find(kid)?,
            None if set.keys.len() == 1 => &set.keys[0],
            None => return None,
        };
        DecodingKey::from_jwk(jwk).ok()
    }

    async fn fetch_jwks(&self) -> Result<JwkSet> {
        let uri = self
            .discovery
            .jwks_uri
            .as_ref()
            .ok_or_else(|| anyhow!("Provider discovery document has no jwks_uri"))?;
        let response = self
            .http_client
            .get(uri)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to fetch JWKS: {}", e))?;
        if !response.status().is_success() {
            return Err(anyhow!("JWKS request failed with status {}", response.status()));
        }
        response
            .json::<JwkSet>()
            .await
            .map_err(|e| anyhow!("Failed to parse JWKS: {}", e))
    }

    pub async fn get_user_info(&self, access_token: &str) -> Result<OidcUserInfo> {
        let endpoint = self
            .discovery
            .userinfo_endpoint
            .as_ref()
            .ok_or_else(|| anyhow!("Provider has no userinfo endpoint"))?;
        let response = self
            .http_client
            .get(endpoint)
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

/// `email_verified` is a boolean per spec, but some providers send a string.
fn parse_bool_claim(value: &serde_json::Value) -> Option<bool> {
    match value {
        serde_json::Value::Bool(b) => Some(*b),
        serde_json::Value::String(s) => Some(s.eq_ignore_ascii_case("true")),
        _ => None,
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

    #[test]
    fn pkce_challenge_matches_rfc7636_example() {
        // RFC 7636 Appendix B
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(pkce_challenge(verifier), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn random_values_are_unique_and_long() {
        let a = random_urlsafe(32);
        let b = random_urlsafe(32);
        assert_ne!(a, b);
        assert!(a.len() >= 43);
    }

    #[test]
    fn email_verified_parsing() {
        assert_eq!(parse_bool_claim(&serde_json::json!(true)), Some(true));
        assert_eq!(parse_bool_claim(&serde_json::json!("true")), Some(true));
        assert_eq!(parse_bool_claim(&serde_json::json!("false")), Some(false));
        assert_eq!(parse_bool_claim(&serde_json::json!(1)), None);
    }
}
