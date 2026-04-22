/*!
 * API Keys Integration Tests
 *
 * Covers the end-to-end behavior of the personal API key feature:
 * - Create → returns plaintext exactly once; subsequent reads do not contain it
 * - Auth with an API key in `Authorization: Bearer` succeeds on `/api/auth/me`
 * - Revocation, expiration, and bogus keys all return 401
 * - Per-user ownership and admin-level cross-user oversight
 * - Role changes (demotion) take effect on the next request
 * - Max-keys-per-user cap returns 409
 * - Rate limit triggers after the 10th creation in an hour
 * - JWT auth still works unchanged
 * - Unit test for SHA-256 stability
 */

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use readur::auth::sha256_hex;
use readur::test_utils::{TestAuthHelper, TestContext};

struct Ctx {
    ctx: TestContext,
    auth: TestAuthHelper,
}

impl Ctx {
    async fn new() -> Self {
        let ctx = TestContext::new().await;
        let auth = TestAuthHelper::new(ctx.app().clone());
        Self { ctx, auth }
    }

    async fn send(&self, method: &str, uri: &str, token: Option<&str>, body: Option<Value>) -> (StatusCode, Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(t) = token {
            builder = builder.header("Authorization", format!("Bearer {}", t));
        }
        let body = match body {
            Some(v) => {
                builder = builder.header("Content-Type", "application/json");
                Body::from(serde_json::to_vec(&v).unwrap())
            }
            None => Body::empty(),
        };
        let response = self.ctx.app().clone().oneshot(builder.body(body).unwrap()).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: Value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };
        (status, json)
    }
}

/// Register + log in a normal user, return their JWT.
async fn user_jwt(ctx: &Ctx) -> (String, uuid::Uuid) {
    let u = ctx.auth.create_test_user().await;
    let tok = ctx.auth.login_user(&u.username, &u.password).await;
    (tok, u.user_response.id)
}

/// Register + log in an admin, return their JWT.
async fn admin_jwt(ctx: &Ctx) -> (String, uuid::Uuid) {
    let a = ctx.auth.create_admin_user().await;
    let tok = ctx.auth.login_user(&a.username, "adminpass123").await;
    (tok, a.user_response.id)
}

async fn create_key(ctx: &Ctx, jwt: &str, name: &str, expires_in_days: Option<u32>) -> Value {
    let body = match expires_in_days {
        Some(d) => json!({ "name": name, "expires_in_days": d }),
        None => json!({ "name": name }),
    };
    let (status, body) = ctx.send("POST", "/api/auth/keys", Some(jwt), Some(body)).await;
    assert_eq!(status, StatusCode::OK, "create failed: {body:?}");
    body
}

// ────────────────────────────────────────────────────────────────────────────
// Unit test (no network): SHA-256 is deterministic and matches a known value
// ────────────────────────────────────────────────────────────────────────────

#[test]
fn sha256_hex_matches_known_vector() {
    // RFC 6234 test vector: sha256("abc")
    assert_eq!(
        sha256_hex("abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    // Stability — same input, same output
    let a = sha256_hex("readur_pat_hello");
    let b = sha256_hex("readur_pat_hello");
    assert_eq!(a, b);
    // Output is always 64 hex chars
    assert_eq!(a.len(), 64);
    assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
}

// ────────────────────────────────────────────────────────────────────────────
// Integration tests
// ────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_returns_plaintext_once_and_list_omits_it() {
    let ctx = Ctx::new().await;
    let (jwt, _) = user_jwt(&ctx).await;

    let created = create_key(&ctx, &jwt, "my-script", Some(30)).await;
    let plaintext = created["plaintext"].as_str().expect("plaintext present on create");
    assert!(plaintext.starts_with("readur_pat_"));
    // 256 bits of base64url-unpadded ≈ 43 chars; plus the 11-char prefix = 54.
    assert!(plaintext.len() >= 50);

    // List must not contain the plaintext anywhere.
    let (status, listed) = ctx.send("GET", "/api/auth/keys", Some(&jwt), None).await;
    assert_eq!(status, StatusCode::OK);
    let serialized = serde_json::to_string(&listed).unwrap();
    assert!(!serialized.contains(plaintext), "plaintext leaked into list response");
    assert!(!serialized.to_lowercase().contains("key_hash"), "hash leaked into list response");

    // The listed entry has the prefix we expect.
    let entries = listed.as_array().unwrap();
    assert_eq!(entries.len(), 1);
    let prefix = entries[0]["key_prefix"].as_str().unwrap();
    assert!(plaintext.starts_with(prefix));
}

#[tokio::test]
async fn api_key_authenticates_like_a_jwt() {
    let ctx = Ctx::new().await;
    let (jwt, uid) = user_jwt(&ctx).await;

    let plaintext = create_key(&ctx, &jwt, "test", Some(30)).await["plaintext"]
        .as_str().unwrap().to_string();

    // Use the API key to hit /api/auth/me — same endpoint JWTs hit.
    let (status, me) = ctx.send("GET", "/api/auth/me", Some(&plaintext), None).await;
    assert_eq!(status, StatusCode::OK, "me failed: {me:?}");
    assert_eq!(me["id"].as_str().unwrap(), uid.to_string());
}

#[tokio::test]
async fn revoked_key_is_rejected() {
    let ctx = Ctx::new().await;
    let (jwt, _) = user_jwt(&ctx).await;

    let created = create_key(&ctx, &jwt, "temp", Some(30)).await;
    let plaintext = created["plaintext"].as_str().unwrap().to_string();
    let key_id = created["api_key"]["id"].as_str().unwrap().to_string();

    // Revoke using JWT auth (the management UI path).
    let (status, _) = ctx.send("DELETE", &format!("/api/auth/keys/{}", key_id), Some(&jwt), None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // The plaintext no longer authenticates.
    let (status, _) = ctx.send("GET", "/api/auth/me", Some(&plaintext), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn garbage_api_key_returns_401_not_500() {
    let ctx = Ctx::new().await;

    // Invalid bogus key with the right prefix
    let (status, _) = ctx.send("GET", "/api/auth/me", Some("readur_pat_this_is_not_a_real_key"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Prefix but hex-valid-looking garbage
    let (status, _) = ctx.send("GET", "/api/auth/me", Some("readur_pat_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn non_owner_cannot_revoke_someone_elses_key() {
    let ctx = Ctx::new().await;
    let (alice_jwt, _) = user_jwt(&ctx).await;
    let (bob_jwt, _) = user_jwt(&ctx).await;

    let alice_key = create_key(&ctx, &alice_jwt, "alice-key", Some(30)).await;
    let key_id = alice_key["api_key"]["id"].as_str().unwrap().to_string();

    // Bob tries to revoke Alice's key — should return 404 (we don't leak existence).
    let (status, _) = ctx.send("DELETE", &format!("/api/auth/keys/{}", key_id), Some(&bob_jwt), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Alice's key still works.
    let plaintext = alice_key["plaintext"].as_str().unwrap().to_string();
    let (status, _) = ctx.send("GET", "/api/auth/me", Some(&plaintext), None).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn admin_can_revoke_any_key_and_list_all_keys() {
    let ctx = Ctx::new().await;
    let (user_jwt_str, _) = user_jwt(&ctx).await;
    let (admin_jwt_str, _) = admin_jwt(&ctx).await;

    let user_key = create_key(&ctx, &user_jwt_str, "user-key", Some(30)).await;
    let user_plaintext = user_key["plaintext"].as_str().unwrap().to_string();
    let user_key_id = user_key["api_key"]["id"].as_str().unwrap().to_string();

    // Admin revokes the user's key.
    let (status, _) = ctx
        .send("DELETE", &format!("/api/auth/keys/{}", user_key_id), Some(&admin_jwt_str), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // The user's plaintext no longer works.
    let (status, _) = ctx.send("GET", "/api/auth/me", Some(&user_plaintext), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // Admin ?all=true returns the revoked key too.
    let (status, listed) = ctx.send("GET", "/api/auth/keys?all=true", Some(&admin_jwt_str), None).await;
    assert_eq!(status, StatusCode::OK);
    let arr = listed.as_array().unwrap();
    assert!(arr.iter().any(|k| k["id"].as_str() == Some(&user_key_id)));
}

#[tokio::test]
async fn demotion_takes_effect_immediately_for_existing_key() {
    let ctx = Ctx::new().await;
    let (admin_jwt_str, admin_uid) = admin_jwt(&ctx).await;

    // Admin issues themselves an API key while they are still admin.
    let plaintext = create_key(&ctx, &admin_jwt_str, "admin-key", Some(30)).await
        ["plaintext"].as_str().unwrap().to_string();

    // Listing all API keys as admin using the API key itself should work (admin).
    let (status, _) = ctx.send("GET", "/api/auth/keys?all=true", Some(&plaintext), None).await;
    assert_eq!(status, StatusCode::OK);

    // Demote the admin directly in the DB — simulates role change via user management.
    sqlx::query("UPDATE users SET role = 'user' WHERE id = $1")
        .bind(admin_uid)
        .execute(ctx.ctx.state().db.get_pool())
        .await
        .expect("demotion sql");

    // Same API key now makes the user a regular user. `?all=true` is silently
    // treated as `?all=false` per the handler contract, so it returns only
    // that user's own keys rather than 403'ing — which is still strictly
    // narrower than the admin view and proves the role re-check happened.
    let (status, listed) = ctx.send("GET", "/api/auth/keys?all=true", Some(&plaintext), None).await;
    assert_eq!(status, StatusCode::OK);
    // Should see only the demoted user's own key (the one we created).
    let arr = listed.as_array().unwrap();
    assert!(arr.iter().all(|k| k["user_id"].as_str() == Some(&admin_uid.to_string())));
}

#[tokio::test]
async fn rejects_when_max_keys_reached() {
    // The creation rate limiter caps API-driven creation at 10/hour, so we
    // seed the extra keys directly through the DB layer and then verify the
    // max-keys cap fires on the API path for the 21st attempt.
    let ctx = Ctx::new().await;
    let (jwt, uid) = user_jwt(&ctx).await;

    let db = &ctx.ctx.state().db;
    for i in 0..20 {
        // Unique dummy hashes so we don't hit the UNIQUE constraint on key_hash.
        let fake_hash = format!("{:064x}", i as u128);
        db.create_api_key(uid, &format!("seed-{i}"), &fake_hash, "readur_pat_X", None)
            .await
            .expect("seed insert");
    }

    let (status, body) = ctx
        .send("POST", "/api/auth/keys", Some(&jwt), Some(json!({"name": "too-many"})))
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "expected 409 got {status}: {body:?}");
}

#[tokio::test]
async fn expired_keys_do_not_count_toward_cap() {
    // Seed 20 keys and backdate their expirations so none are usable. The
    // 21st creation should still succeed because expired keys don't count
    // against the per-user active cap.
    let ctx = Ctx::new().await;
    let (jwt, uid) = user_jwt(&ctx).await;

    let db = &ctx.ctx.state().db;
    for i in 0..20 {
        let fake_hash = format!("{:064x}", i as u128);
        let past = chrono::Utc::now() - chrono::Duration::hours(1);
        db.create_api_key(uid, &format!("expired-{i}"), &fake_hash, "readur_pat_X", Some(past))
            .await
            .expect("seed insert");
    }

    let (status, body) = ctx
        .send("POST", "/api/auth/keys", Some(&jwt), Some(json!({"name": "fresh"})))
        .await;
    assert_eq!(status, StatusCode::OK, "expected 200 got {status}: {body:?}");
}

#[tokio::test]
async fn rate_limit_triggers_after_many_creations() {
    let ctx = Ctx::new().await;
    let (jwt, _) = user_jwt(&ctx).await;

    // 10/hour per user. Make 11 quickly and expect the 11th to be rate limited.
    let mut saw_429 = false;
    for i in 0..11 {
        let (status, _) = ctx
            .send("POST", "/api/auth/keys", Some(&jwt), Some(json!({ "name": format!("k{}", i) })))
            .await;
        if status == StatusCode::TOO_MANY_REQUESTS {
            saw_429 = true;
            break;
        }
    }
    assert!(saw_429, "expected a 429 within 11 rapid creations");
}

#[tokio::test]
async fn rejects_zero_or_oversized_expires_in_days() {
    let ctx = Ctx::new().await;
    let (jwt, _) = user_jwt(&ctx).await;

    let (status, _) = ctx
        .send("POST", "/api/auth/keys", Some(&jwt), Some(json!({"name": "zero", "expires_in_days": 0})))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = ctx
        .send("POST", "/api/auth/keys", Some(&jwt), Some(json!({"name": "too-long", "expires_in_days": 9999})))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = ctx
        .send("POST", "/api/auth/keys", Some(&jwt), Some(json!({"name": ""})))
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn expired_key_is_rejected() {
    let ctx = Ctx::new().await;
    let (jwt, _) = user_jwt(&ctx).await;

    let created = create_key(&ctx, &jwt, "soon-to-expire", Some(30)).await;
    let plaintext = created["plaintext"].as_str().unwrap().to_string();
    let key_id = created["api_key"]["id"].as_str().unwrap().to_string();

    // Backdate expiration in the DB so the key is already expired.
    sqlx::query("UPDATE api_keys SET expires_at = NOW() - INTERVAL '1 hour' WHERE id = $1::uuid")
        .bind(&key_id)
        .execute(ctx.ctx.state().db.get_pool())
        .await
        .expect("backdate expires_at");

    let (status, _) = ctx.send("GET", "/api/auth/me", Some(&plaintext), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn jwt_auth_still_works_alongside_api_keys() {
    let ctx = Ctx::new().await;
    let (jwt, _) = user_jwt(&ctx).await;

    // JWTs continue to authenticate /api/auth/me just as before.
    let (status, me) = ctx.send("GET", "/api/auth/me", Some(&jwt), None).await;
    assert_eq!(status, StatusCode::OK, "me failed: {me:?}");
    assert!(me["id"].is_string());
}
