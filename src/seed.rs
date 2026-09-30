use anyhow::{Context, Result};
use tracing::{info, warn};
use std::env;
use std::path::{Path, PathBuf};
use crate::db::Database;
use crate::models::CreateUser;
use crate::utils::security::generate_secure_password;

/// File name of the generated first-run admin password.
pub const INITIAL_ADMIN_PASSWORD_FILE: &str = "initial-admin-password";

/// Directory inside `UPLOAD_PATH` that holds the generated password file.
/// A subdirectory keeps it apart from files the server manages in the
/// upload root.
pub const INITIAL_ADMIN_PASSWORD_DIR: &str = ".readur";

/// Where a generated admin password is written: `ADMIN_PASSWORD_FILE` if set,
/// otherwise `<UPLOAD_PATH>/.readur/initial-admin-password`. The upload
/// directory is used because the server can always write there; uploaded
/// files are only served through the API, never from this directory.
pub fn initial_admin_password_path(upload_path: &str) -> PathBuf {
    if let Ok(path) = env::var("ADMIN_PASSWORD_FILE") {
        if !path.trim().is_empty() {
            return PathBuf::from(path);
        }
    }
    Path::new(upload_path)
        .join(INITIAL_ADMIN_PASSWORD_DIR)
        .join(INITIAL_ADMIN_PASSWORD_FILE)
}

/// Default admin email: `ADMIN_EMAIL`, or `<username>@localhost`.
pub fn admin_email(admin_username: &str) -> String {
    env::var("ADMIN_EMAIL")
        .ok()
        .filter(|e| !e.trim().is_empty())
        .unwrap_or_else(|| format!("{}@localhost", admin_username))
}

/// Write a new secret file with mode 0600. An existing file (or a symlink
/// at the path) is never overwritten: `create_new` opens with
/// `O_CREAT | O_EXCL`, which fails if anything already exists at the path,
/// including a symlink.
fn write_secret_file(path: &Path, contents: &str) -> Result<()> {
    use std::io::Write;

    if let Some(dir) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(dir)?;
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            anyhow::anyhow!(
                "{} already exists; it may hold a previously generated password. \
                 Remove it, or set ADMIN_PASSWORD",
                path.display()
            )
        } else {
            e.into()
        }
    })?;
    writeln!(file, "{}", contents)?;
    Ok(())
}

/// Create the initial admin account if it does not exist.
///
/// The password comes from `ADMIN_PASSWORD`. When that is unset a random
/// password is generated and written to [`initial_admin_password_path`]
/// (mode 0600); it is never printed or logged.
pub async fn seed_admin_user(db: &Database, upload_path: &str) -> Result<()> {
    let admin_username = env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let admin_email = admin_email(&admin_username);

    match db.get_user_by_username(&admin_username).await {
        Ok(Some(_)) => {
            info!("✅ Admin user '{}' already exists", admin_username);
            info!("💡 To reset the admin password, run: readur reset-admin-password");
            return Ok(());
        }
        Ok(None) => {}
        Err(e) => {
            return Err(e).context("Failed to check whether the admin user exists");
        }
    }

    let (admin_password, password_file) = match env::var("ADMIN_PASSWORD") {
        Ok(pwd) => {
            if pwd.len() < 8 {
                anyhow::bail!("ADMIN_PASSWORD must be at least 8 characters long");
            }
            (pwd, None)
        }
        Err(_) => {
            let pwd = generate_secure_password(24);
            let path = initial_admin_password_path(upload_path);
            // Persist before creating the account so a failure here never
            // leaves an admin whose password nobody knows.
            write_secret_file(&path, &pwd).with_context(|| {
                format!(
                    "Failed to write the generated admin password to {}. Set ADMIN_PASSWORD, \
                     or ADMIN_PASSWORD_FILE to a writable location",
                    path.display()
                )
            })?;
            (pwd, Some(path))
        }
    };

    let create_user = CreateUser {
        username: admin_username.clone(),
        email: admin_email.clone(),
        password: admin_password,
        role: Some(crate::models::UserRole::Admin),
    };

    match db.create_user(create_user).await {
        Ok(user) => {
            info!("✅ Created admin user '{}' ({}, id {})", admin_username, admin_email, user.id);
            match password_file {
                Some(path) => info!(
                    "🔑 The generated admin password was written to {} - sign in, change it, then delete the file",
                    path.display()
                ),
                None => info!("🔑 Admin password taken from ADMIN_PASSWORD"),
            }
            info!("💡 To reset the admin password later, run: readur reset-admin-password");
        }
        Err(e) => {
            warn!("❌ Failed to create admin user: {}", e);
            if let Some(path) = password_file {
                if let Err(remove_err) = std::fs::remove_file(&path) {
                    warn!(
                        "Failed to remove the unused admin password file {}: {}",
                        path.display(),
                        remove_err
                    );
                }
            }
        }
    }

    Ok(())
}
