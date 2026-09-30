//! Admin accounts for tests. Public registration always creates standard
//! users, so tests create admins through the admin-only `POST /api/users`
//! endpoint, signed in as an admin seeded outside the public API.

use serde_json::json;

/// Admin account seeded into every `TestContext` database. Public
/// registration only creates standard users, so tests create further admins
/// through `POST /api/users` while signed in as this account.
pub const TEST_BOOTSTRAP_ADMIN_USERNAME: &str = "test_bootstrap_admin";

/// Password of the bootstrap admin: `ADMIN_PASSWORD` from the environment when
/// set and non-empty, otherwise a random value generated once per process.
pub fn bootstrap_admin_password() -> &'static str {
    static PASSWORD: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    PASSWORD.get_or_init(|| match std::env::var("ADMIN_PASSWORD") {
        Ok(p) if !p.is_empty() => p,
        _ => uuid::Uuid::new_v4().simple().to_string(),
    })
}

/// Insert the bootstrap admin directly, hashing its password once per process
/// at the minimum bcrypt cost to keep test setup fast.
pub(crate) async fn seed_bootstrap_admin(db: &crate::db::Database) -> anyhow::Result<()> {
    static HASH: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let hash = match HASH.get() {
        Some(hash) => hash.clone(),
        None => {
            let hash = bcrypt::hash(bootstrap_admin_password(), 4)?;
            HASH.get_or_init(|| hash).clone()
        }
    };
    sqlx::query(
        "INSERT INTO users (username, email, password_hash, role, auth_provider) \
         VALUES ($1, $2, $3, 'admin', 'local') ON CONFLICT (username) DO NOTHING",
    )
    .bind(TEST_BOOTSTRAP_ADMIN_USERNAME)
    .bind(format!("{}@example.com", TEST_BOOTSTRAP_ADMIN_USERNAME))
    .bind(hash)
    .execute(&db.pool)
    .await?;
    Ok(())
}

/// Create a user on a running readur server at `base_url`, returning the
/// server's response to the create request.
///
/// Public registration always creates standard users, so a user whose `role`
/// is `admin` is created through the admin-only `POST /api/users` endpoint,
/// signed in as the server's seeded admin (`ADMIN_USERNAME`, default `admin`,
/// and `ADMIN_PASSWORD`, which must match the server's environment). Any other
/// user goes through `POST /api/auth/register`.
pub async fn register_user_on_server<T: serde::Serialize>(
    client: &reqwest::Client,
    base_url: &str,
    user: &T,
) -> anyhow::Result<reqwest::Response> {
    const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
    let body = serde_json::to_value(user)?;
    if body.get("role").and_then(|r| r.as_str()) != Some("admin") {
        return Ok(client
            .post(format!("{}/api/auth/register", base_url))
            .json(&body)
            .timeout(TIMEOUT)
            .send()
            .await?);
    }

    let admin_username = std::env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let admin_password = std::env::var("ADMIN_PASSWORD").map_err(|_| {
        anyhow::anyhow!(
            "creating an admin needs the server's seeded admin: set ADMIN_PASSWORD (and ADMIN_USERNAME if not 'admin') to match the server"
        )
    })?;
    let login = client
        .post(format!("{}/api/auth/login", base_url))
        .json(&json!({ "username": admin_username, "password": admin_password }))
        .timeout(TIMEOUT)
        .send()
        .await?;
    if !login.status().is_success() {
        anyhow::bail!(
            "seeded admin login failed ({}): {}",
            login.status(),
            login.text().await.unwrap_or_default()
        );
    }
    let login: serde_json::Value = login.json().await?;
    let token = login["token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("seeded admin login returned no token"))?;

    Ok(client
        .post(format!("{}/api/users", base_url))
        .bearer_auth(token)
        .json(&body)
        .timeout(TIMEOUT)
        .send()
        .await?)
}
