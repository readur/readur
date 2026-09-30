//! OIDC login flow tests against a mock identity provider.
//!
//! The provider signs ID tokens with HS256 keyed by the client secret, which
//! the relying party accepts for confidential clients.

use axum::{body::Body, http::Request, http::StatusCode, Router};
use base64ct::Encoding;
use jsonwebtoken::{encode, EncodingKey, Header};
use readur::models::{CreateUser, UserRole};
use readur::oidc::OidcClient;
use readur::test_utils::TestContext;
use readur::AppState;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tower::util::ServiceExt;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

const CLIENT_ID: &str = "test-client-id";
const CLIENT_SECRET: &str = "test-client-secret-with-enough-entropy";
const REDIRECT_URI: &str = "http://localhost:8000/api/auth/oidc/callback";
const STATE_COOKIE: &str = "readur_oidc_state";

struct OidcTestApp {
    app: Router,
    state: Arc<AppState>,
    provider: MockServer,
    // Keeps the database container alive for the duration of the test.
    _ctx: TestContext,
}

async fn setup_with(configure: impl FnOnce(&mut readur::config::Config)) -> OidcTestApp {
    let ctx = TestContext::new().await;
    let provider = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "issuer": provider.uri(),
            "authorization_endpoint": format!("{}/auth", provider.uri()),
            "token_endpoint": format!("{}/token", provider.uri()),
        })))
        .mount(&provider)
        .await;

    let mut config = ctx.state.config.clone();
    config.oidc_enabled = true;
    config.oidc_client_id = Some(CLIENT_ID.to_string());
    config.oidc_client_secret = Some(CLIENT_SECRET.to_string());
    config.oidc_issuer_url = Some(provider.uri());
    config.oidc_redirect_uri = Some(REDIRECT_URI.to_string());
    config.oidc_auto_register = Some(true);
    configure(&mut config);

    let oidc_client = OidcClient::new(&config).await.expect("OIDC client should initialise");

    let state = Arc::new(AppState {
        db: ctx.state.db.clone(),
        config,
        file_service: ctx.state.file_service.clone(),
        webdav_scheduler: None,
        source_scheduler: None,
        queue_service: ctx.state.queue_service.clone(),
        oidc_client: Some(Arc::new(oidc_client)),
        sync_progress_tracker: ctx.state.sync_progress_tracker.clone(),
        user_watch_service: None,
        webdav_metrics_collector: None,
        rate_limiters: readur::rate_limit::RateLimiters::new(),
    });

    let app = Router::new()
        .nest("/api/auth", readur::routes::auth::router())
        .with_state(state.clone());

    OidcTestApp { app, state, provider, _ctx: ctx }
}

async fn setup() -> OidcTestApp {
    setup_with(|_| {}).await
}

/// Values the relying party put into the authorization request.
struct LoginStart {
    state: String,
    nonce: String,
    code_challenge: String,
    cookie_value: String,
}

async fn send(app: &Router, request: Request<Body>) -> axum::response::Response {
    app.clone().oneshot(request).await.unwrap()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap_or(Value::Null)
}

fn location(response: &axum::response::Response) -> String {
    response
        .headers()
        .get("location")
        .expect("redirect should carry a Location header")
        .to_str()
        .unwrap()
        .to_string()
}

async fn start_login(t: &OidcTestApp) -> LoginStart {
    let response = send(
        &t.app,
        Request::get("/api/auth/oidc/login").body(Body::empty()).unwrap(),
    )
    .await;
    assert!(response.status().is_redirection(), "login should redirect, got {}", response.status());

    let set_cookie = response
        .headers()
        .get("set-cookie")
        .expect("login should set the state cookie")
        .to_str()
        .unwrap()
        .to_string();
    assert!(set_cookie.starts_with(&format!("{}=", STATE_COOKIE)));
    assert!(set_cookie.contains("HttpOnly"));
    assert!(set_cookie.contains("SameSite=Lax"));
    let cookie_value = set_cookie
        .split(';')
        .next()
        .unwrap()
        .split_once('=')
        .unwrap()
        .1
        .to_string();

    let url = url::Url::parse(&location(&response)).unwrap();
    assert!(url.as_str().starts_with(&format!("{}/auth", t.provider.uri())));
    let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
    assert_eq!(params["client_id"], CLIENT_ID);
    assert_eq!(params["response_type"], "code");
    assert_eq!(params["redirect_uri"], REDIRECT_URI);
    assert!(params["scope"].split(' ').any(|s| s == "openid"));
    assert_eq!(params["code_challenge_method"], "S256");

    LoginStart {
        state: params["state"].clone(),
        nonce: params["nonce"].clone(),
        code_challenge: params["code_challenge"].clone(),
        cookie_value,
    }
}

fn id_token(t: &OidcTestApp, claims: Value) -> String {
    let now = chrono::Utc::now().timestamp();
    let mut full = json!({
        "iss": t.provider.uri(),
        "aud": CLIENT_ID,
        "iat": now,
        "exp": now + 300,
    });
    for (k, v) in claims.as_object().unwrap() {
        full[k] = v.clone();
    }
    encode(&Header::default(), &full, &EncodingKey::from_secret(CLIENT_SECRET.as_bytes())).unwrap()
}

async fn mock_token_endpoint(t: &OidcTestApp, id_token: String) {
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "provider-access-token",
            "token_type": "Bearer",
            "expires_in": 300,
            "id_token": id_token,
        })))
        // One response per login so later mocks in the same test take over.
        .up_to_n_times(1)
        .mount(&t.provider)
        .await;
}

async fn callback(t: &OidcTestApp, query: &str, cookie: Option<&str>) -> axum::response::Response {
    let mut builder = Request::get(format!("/api/auth/oidc/callback?{}", query));
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", format!("{}={}", STATE_COOKIE, cookie));
    }
    send(&t.app, builder.body(Body::empty()).unwrap()).await
}

async fn exchange(t: &OidcTestApp, code: &str) -> axum::response::Response {
    send(
        &t.app,
        Request::post("/api/auth/oidc/exchange")
            .header("content-type", "application/json")
            .body(Body::from(json!({ "code": code }).to_string()))
            .unwrap(),
    )
    .await
}

fn unique(prefix: &str) -> String {
    format!("{}{}", prefix, &uuid::Uuid::new_v4().simple().to_string()[..10])
}

#[tokio::test]
async fn login_redirect_sets_state_cookie_and_pkce() {
    let t = setup().await;
    let first = start_login(&t).await;
    assert_eq!(first.cookie_value, first.state);
    assert!(!first.nonce.is_empty());
    assert!(!first.code_challenge.is_empty());

    let second = start_login(&t).await;
    assert_ne!(first.state, second.state);
    assert_ne!(first.nonce, second.nonce);
}

#[tokio::test]
async fn login_without_oidc_configured_is_rejected() {
    let ctx = TestContext::new().await;
    let response = send(
        &ctx.app,
        Request::get("/api/auth/oidc/login").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn callback_happy_path_creates_user_and_issues_single_use_code() {
    let t = setup().await;
    let login = start_login(&t).await;

    let subject = unique("sub-");
    let username = unique("oidcuser_");
    let email = format!("{}@example.com", username);
    mock_token_endpoint(
        &t,
        id_token(
            &t,
            json!({
                "sub": subject,
                "nonce": login.nonce,
                "email": email,
                "email_verified": true,
                "preferred_username": username,
            }),
        ),
    )
    .await;

    let response = callback(
        &t,
        &format!("code=provider-code&state={}", login.state),
        Some(&login.cookie_value),
    )
    .await;
    let status = response.status();
    assert!(status.is_redirection(), "expected redirect, got {} {:?}", status, body_json(response).await);

    // The state cookie is cleared on the way out.
    let cleared = response.headers().get("set-cookie").unwrap().to_str().unwrap().to_string();
    assert!(cleared.contains("Max-Age=0"));

    let target = location(&response);
    let prefix = "http://localhost:8000/auth/callback#code=";
    assert!(target.starts_with(prefix), "unexpected redirect target {}", target);
    let code = &target[prefix.len()..];
    assert!(!code.is_empty());

    // PKCE: the token request carried the verifier matching the challenge.
    let requests = t.provider.received_requests().await.unwrap();
    let token_request = requests.iter().find(|r| r.url.path() == "/token").unwrap();
    let form: HashMap<String, String> = url::form_urlencoded::parse(&token_request.body).into_owned().collect();
    assert_eq!(form["grant_type"], "authorization_code");
    assert_eq!(form["code"], "provider-code");
    let verifier = &form["code_verifier"];
    let challenge = base64ct::Base64UrlUnpadded::encode_string(&Sha256::digest(verifier.as_bytes()));
    assert_eq!(challenge, login.code_challenge);

    // Redeem the handoff code for a session.
    let response = exchange(&t, code).await;
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let token = body["token"].as_str().expect("exchange should return a token");
    assert!(!token.is_empty());
    assert_eq!(body["user"]["username"], username.as_str());

    let me = send(
        &t.app,
        Request::get("/api/auth/me")
            .header("authorization", format!("Bearer {}", token))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(me.status(), StatusCode::OK);

    // The code is single use.
    let again = exchange(&t, code).await;
    assert_eq!(again.status(), StatusCode::UNAUTHORIZED);

    let user = t.state.db.get_user_by_username(&username).await.unwrap().unwrap();
    assert_eq!(user.email, email);
    assert_eq!(user.oidc_subject.as_deref(), Some(subject.as_str()));
    assert_eq!(user.role, UserRole::User);

    // The login state is single use as well.
    let replay = callback(
        &t,
        &format!("code=provider-code&state={}", login.state),
        Some(&login.cookie_value),
    )
    .await;
    assert_eq!(replay.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn callback_requires_matching_state_cookie() {
    let t = setup().await;
    let login = start_login(&t).await;

    let missing = callback(&t, &format!("code=c&state={}", login.state), None).await;
    assert_eq!(missing.status(), StatusCode::BAD_REQUEST);

    let other = start_login(&t).await;
    let mismatched = callback(
        &t,
        &format!("code=c&state={}", login.state),
        Some(&other.cookie_value),
    )
    .await;
    assert_eq!(mismatched.status(), StatusCode::BAD_REQUEST);

    let no_state = callback(&t, "code=c", Some(&login.cookie_value)).await;
    assert_eq!(no_state.status(), StatusCode::BAD_REQUEST);

    let no_code = callback(&t, &format!("state={}", login.state), Some(&login.cookie_value)).await;
    assert_eq!(no_code.status(), StatusCode::BAD_REQUEST);

    // An unknown state is rejected even when cookie and query agree.
    let unknown = callback(&t, "code=c&state=not-issued", Some("not-issued")).await;
    assert_eq!(unknown.status(), StatusCode::BAD_REQUEST);

    // None of the above reached the token endpoint.
    let requests = t.provider.received_requests().await.unwrap();
    assert!(requests.iter().all(|r| r.url.path() != "/token"));
}

#[tokio::test]
async fn callback_rejects_wrong_nonce() {
    let t = setup().await;
    let login = start_login(&t).await;
    let username = unique("nonceuser_");

    mock_token_endpoint(
        &t,
        id_token(
            &t,
            json!({
                "sub": unique("sub-"),
                "nonce": "some-other-nonce",
                "email": format!("{}@example.com", username),
                "preferred_username": username,
            }),
        ),
    )
    .await;

    let response = callback(
        &t,
        &format!("code=c&state={}", login.state),
        Some(&login.cookie_value),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(t.state.db.get_user_by_username(&username).await.unwrap().is_none());
}

#[tokio::test]
async fn callback_rejects_token_response_without_valid_id_token() {
    let t = setup().await;

    // No id_token at all.
    let login = start_login(&t).await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "provider-access-token",
            "token_type": "Bearer",
        })))
        .up_to_n_times(1)
        .mount(&t.provider)
        .await;
    let response = callback(&t, &format!("code=c&state={}", login.state), Some(&login.cookie_value)).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // id_token for a different audience.
    let login = start_login(&t).await;
    mock_token_endpoint(
        &t,
        id_token(&t, json!({ "sub": unique("sub-"), "nonce": login.nonce, "aud": "someone-else" })),
    )
    .await;
    let response = callback(&t, &format!("code=c&state={}", login.state), Some(&login.cookie_value)).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn callback_does_not_link_existing_local_account_by_default() {
    let t = setup().await;
    let username = unique("localuser_");
    let email = format!("{}@example.com", username);
    let local = t
        .state
        .db
        .create_user(CreateUser {
            username: username.clone(),
            email: email.clone(),
            password: "localpassword123".to_string(),
            role: Some(UserRole::Admin),
        })
        .await
        .unwrap();

    let login = start_login(&t).await;
    mock_token_endpoint(
        &t,
        id_token(
            &t,
            json!({
                "sub": unique("sub-"),
                "nonce": login.nonce,
                "email": email,
                "email_verified": true,
                "preferred_username": username,
            }),
        ),
    )
    .await;

    let response = callback(&t, &format!("code=c&state={}", login.state), Some(&login.cookie_value)).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let after = t.state.db.get_user_by_id(local.id).await.unwrap().unwrap();
    assert!(after.oidc_subject.is_none());
    assert!(after.oidc_issuer.is_none());
}

#[tokio::test]
async fn callback_links_verified_email_when_enabled() {
    let t = setup_with(|c| c.security.oidc_link_existing_by_email = true).await;
    let username = unique("linkuser_");
    let email = format!("{}@example.com", username);
    let local = t
        .state
        .db
        .create_user(CreateUser {
            username: username.clone(),
            email: email.clone(),
            password: "localpassword123".to_string(),
            role: Some(UserRole::User),
        })
        .await
        .unwrap();

    // Unverified email is still not linked.
    let login = start_login(&t).await;
    mock_token_endpoint(
        &t,
        id_token(&t, json!({ "sub": unique("sub-"), "nonce": login.nonce, "email": email, "email_verified": false })),
    )
    .await;
    let response = callback(&t, &format!("code=c&state={}", login.state), Some(&login.cookie_value)).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    let subject = unique("sub-");
    let login = start_login(&t).await;
    mock_token_endpoint(
        &t,
        id_token(&t, json!({ "sub": subject, "nonce": login.nonce, "email": email, "email_verified": true })),
    )
    .await;
    let response = callback(&t, &format!("code=c&state={}", login.state), Some(&login.cookie_value)).await;
    assert!(response.status().is_redirection(), "expected redirect, got {}", response.status());

    let after = t.state.db.get_user_by_id(local.id).await.unwrap().unwrap();
    assert_eq!(after.oidc_subject.as_deref(), Some(subject.as_str()));
}

#[tokio::test]
async fn callback_provider_errors_are_unauthorized() {
    let t = setup().await;

    let login = start_login(&t).await;
    let response = callback(
        &t,
        &format!("error=access_denied&state={}", login.state),
        Some(&login.cookie_value),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let login = start_login(&t).await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({ "error": "invalid_grant" })))
        .mount(&t.provider)
        .await;
    let response = callback(&t, &format!("code=bad&state={}", login.state), Some(&login.cookie_value)).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn exchange_rejects_unknown_code() {
    let t = setup().await;
    let response = exchange(&t, "never-issued").await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}
