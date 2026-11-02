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

/// `ADMIN_PASSWORD`, treating an empty or blank value as unset (compose
/// passes `${ADMIN_PASSWORD:-}` through as an empty string).
pub fn configured_admin_password(value: Option<String>) -> Result<Option<String>> {
    match value.filter(|p| !p.trim().is_empty()) {
        Some(pwd) if pwd.len() < 8 => anyhow::bail!("ADMIN_PASSWORD must be at least 8 characters long"),
        other => Ok(other),
    }
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

/// Re-apply owner-only permissions to a generated password file that has not
/// been deleted yet. Deployment tooling may recursively change modes on the
/// upload volume (for example a `chmod -R` in an init container), so this
/// runs on every start for as long as the file exists. The parent directory
/// is tightened too when it is the default `.readur` directory. Symlinks are
/// left alone.
#[cfg(unix)]
pub fn restrict_password_file_permissions(path: &Path, restrict_parent: bool) {
    use std::os::unix::fs::PermissionsExt;

    let is_regular = |p: &Path| std::fs::symlink_metadata(p).map(|m| m.file_type().is_file()).unwrap_or(false);
    let is_dir = |p: &Path| std::fs::symlink_metadata(p).map(|m| m.file_type().is_dir()).unwrap_or(false);
    if !is_regular(path) {
        return;
    }
    let mut targets = vec![(path, 0o600)];
    if let Some(dir) = path.parent().filter(|d| restrict_parent && is_dir(d)) {
        targets.push((dir, 0o700));
    }
    for (target, mode) in targets {
        // Skip targets already at the right mode: a no-op chmod still emits a
        // metadata-change event, which restarts file watchers such as cargo-watch.
        let current = std::fs::metadata(target).map(|m| m.permissions().mode() & 0o777).ok();
        if current == Some(mode) {
            continue;
        }
        if let Err(e) = std::fs::set_permissions(target, std::fs::Permissions::from_mode(mode)) {
            warn!("Failed to restrict permissions of {}: {}", target.display(), e);
        }
    }
    warn!(
        "The generated admin password file {} still exists; delete it once you have signed in and changed the password",
        path.display()
    );
}

#[cfg(not(unix))]
pub fn restrict_password_file_permissions(_path: &Path, _restrict_parent: bool) {}

/// Create the initial admin account if it does not exist.
///
/// The password comes from `ADMIN_PASSWORD`. When that is unset a random
/// password is generated and written to [`initial_admin_password_path`]
/// (mode 0600); it is never printed or logged.
pub async fn seed_admin_user(db: &Database, upload_path: &str) -> Result<()> {
    let password_path = initial_admin_password_path(upload_path);
    let default_dir = Path::new(upload_path).join(INITIAL_ADMIN_PASSWORD_DIR);
    restrict_password_file_permissions(&password_path, password_path.parent() == Some(default_dir.as_path()));

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

    let (admin_password, password_file) = match configured_admin_password(env::var("ADMIN_PASSWORD").ok())? {
        Some(pwd) => (pwd, None),
        None => {
            let pwd = generate_secure_password(24);
            let path = password_path;
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

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn mode(path: &Path) -> u32 {
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[test]
    fn blank_admin_password_counts_as_unset() {
        assert_eq!(configured_admin_password(None).unwrap(), None);
        assert_eq!(configured_admin_password(Some(String::new())).unwrap(), None);
        assert_eq!(configured_admin_password(Some("   ".into())).unwrap(), None);
    }

    #[test]
    fn admin_password_must_be_eight_characters() {
        assert!(configured_admin_password(Some("short".into())).is_err());
        assert_eq!(
            configured_admin_password(Some("longenough".into())).unwrap(),
            Some("longenough".to_string())
        );
    }

    #[test]
    fn leftover_password_file_permissions_are_restricted() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join(INITIAL_ADMIN_PASSWORD_DIR);
        let file = dir.join(INITIAL_ADMIN_PASSWORD_FILE);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, "generated\n").unwrap();
        // What a recursive `chmod -R 755` on the upload volume leaves behind.
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();

        restrict_password_file_permissions(&file, true);
        assert_eq!(mode(&file), 0o600);
        assert_eq!(mode(&dir), 0o700);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "generated\n");
    }

    #[test]
    fn custom_location_parent_and_symlinks_are_left_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("admin-password");
        std::fs::write(&file, "generated\n").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
        std::fs::set_permissions(tmp.path(), std::fs::Permissions::from_mode(0o755)).unwrap();

        restrict_password_file_permissions(&file, false);
        assert_eq!(mode(&file), 0o600);
        assert_eq!(mode(tmp.path()), 0o755);

        let target = tmp.path().join("target");
        std::fs::write(&target, "other\n").unwrap();
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();
        let link = tmp.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        restrict_password_file_permissions(&link, false);
        assert_eq!(mode(&target), 0o644);
    }
}
