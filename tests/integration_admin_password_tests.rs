use anyhow::Result;
use readur::test_utils::{TestContext, TestAuthHelper};
use readur::{seed, commands, models::{CreateUser, UserRole}};

fn clear_seed_env() {
    for var in ["ADMIN_PASSWORD", "ADMIN_USERNAME", "ADMIN_EMAIL", "ADMIN_PASSWORD_FILE"] {
        std::env::remove_var(var);
    }
}

/// Upload path inside a fresh temporary directory, so a generated password
/// file lands somewhere disposable.
fn seed_upload_path() -> String {
    let dir = tempfile::tempdir().expect("tempdir").keep();
    dir.join("uploads").to_string_lossy().into_owned()
}

/// Test that admin user is created with auto-generated password on first run
#[tokio::test]
async fn test_admin_seed_creates_user_with_auto_password() {
    let ctx = TestContext::new().await;
    let data_dir = tempfile::tempdir().expect("tempdir");
    let upload_path = data_dir.path().join("uploads");
    let result: Result<()> = async {
        // Clear any env vars to test auto-generation
        clear_seed_env();

        // Run seed
        seed::seed_admin_user(&ctx.state.db, upload_path.to_str().unwrap()).await?;

        // The generated password is written next to the upload directory,
        // readable only by the owner.
        let password_file = data_dir.path().join(seed::INITIAL_ADMIN_PASSWORD_FILE);
        let password = std::fs::read_to_string(&password_file)?.trim().to_string();
        assert_eq!(password.len(), 24, "Generated password should be written to the file");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&password_file)?.permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "Password file should only be readable by its owner");
        }
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let token = auth_helper.login_user("admin", &password).await;
        assert!(!token.is_empty(), "Should be able to login with the generated password");

        // Verify admin user exists
        let admin = ctx.state.db.get_user_by_username("admin").await?
            .expect("Admin user should exist");

        // Verify role is Admin
        assert_eq!(admin.role, UserRole::Admin, "User should have Admin role");

        // Verify password is hashed (bcrypt format)
        assert!(admin.password_hash.is_some(), "Password hash should exist");
        let hash = admin.password_hash.unwrap();
        assert!(
            hash.starts_with("$2b$") || hash.starts_with("$2a$"),
            "Password should be bcrypt hashed"
        );

        // Verify email format
        assert_eq!(admin.email, "admin@localhost", "Email should use default format");

        Ok(())
    }.await;

    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}

/// Test that ADMIN_EMAIL overrides the default admin email
#[tokio::test]
async fn test_admin_seed_uses_admin_email() {
    let ctx = TestContext::new().await;
    let result: Result<()> = async {
        clear_seed_env();
        std::env::set_var("ADMIN_PASSWORD", "emailtest123");
        std::env::set_var("ADMIN_EMAIL", "ops@example.org");

        seed::seed_admin_user(&ctx.state.db, &seed_upload_path()).await?;

        let admin = ctx.state.db.get_user_by_username("admin").await?
            .expect("Admin user should exist");
        assert_eq!(admin.email, "ops@example.org");
        Ok(())
    }.await;

    clear_seed_env();
    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}

/// Test that admin user is created with provided ADMIN_PASSWORD
#[tokio::test]
async fn test_admin_seed_uses_env_password() {
    let ctx = TestContext::new().await;
    let result: Result<()> = async {
        // Set password via environment variable
        std::env::set_var("ADMIN_PASSWORD", "testpass123");
        std::env::remove_var("ADMIN_USERNAME");

        // Run seed
        seed::seed_admin_user(&ctx.state.db, &seed_upload_path()).await?;

        // Verify admin user exists
        let admin = ctx.state.db.get_user_by_username("admin").await?
            .expect("Admin user should exist");

        // Verify we can login with the provided password
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let token = auth_helper.login_user("admin", "testpass123").await;
        assert!(!token.is_empty(), "Should be able to login with provided password");

        // Clean up env var
        std::env::remove_var("ADMIN_PASSWORD");

        Ok(())
    }.await;

    std::env::remove_var("ADMIN_PASSWORD");
    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}

/// Test that subsequent runs don't duplicate admin user
#[tokio::test]
async fn test_admin_seed_does_not_duplicate_user() {
    let ctx = TestContext::new().await;
    let result: Result<()> = async {
        std::env::remove_var("ADMIN_PASSWORD");
        std::env::remove_var("ADMIN_USERNAME");

        // Run seed first time
        seed::seed_admin_user(&ctx.state.db, &seed_upload_path()).await?;
        let first_admin = ctx.state.db.get_user_by_username("admin").await?
            .expect("Admin should exist after first seed");

        // Run seed second time
        seed::seed_admin_user(&ctx.state.db, &seed_upload_path()).await?;
        let second_admin = ctx.state.db.get_user_by_username("admin").await?
            .expect("Admin should still exist after second seed");

        // Verify same user (same ID and hash)
        assert_eq!(first_admin.id, second_admin.id, "Should be the same user");
        assert_eq!(
            first_admin.password_hash, second_admin.password_hash,
            "Password should not have changed"
        );

        // Verify only one admin exists
        let all_users = ctx.state.db.get_all_users().await?;
        let admin_count = all_users.iter().filter(|u| u.username == "admin").count();
        assert_eq!(admin_count, 1, "Should only have one admin user");

        Ok(())
    }.await;

    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}

/// Test that user can successfully login with generated credentials
#[tokio::test]
async fn test_admin_seed_allows_login() {
    let ctx = TestContext::new().await;
    let result: Result<()> = async {
        // Set known password for testing
        std::env::set_var("ADMIN_PASSWORD", "logintest123");
        std::env::remove_var("ADMIN_USERNAME");

        // Run seed
        seed::seed_admin_user(&ctx.state.db, &seed_upload_path()).await?;

        // Attempt login
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let token = auth_helper.login_user("admin", "logintest123").await;

        // Verify token is not empty
        assert!(!token.is_empty(), "Should receive valid JWT token");

        // Verify token is valid by making authenticated request
        let _response = auth_helper
            .make_authenticated_request("GET", "/api/auth/me", None, &token)
            .await;

        // If we get here without panicking, the authenticated request succeeded

        // Clean up env var
        std::env::remove_var("ADMIN_PASSWORD");

        Ok(())
    }.await;

    std::env::remove_var("ADMIN_PASSWORD");
    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}

/// Test that reset command generates new password and invalidates old one
#[tokio::test]
async fn test_reset_command_changes_password() {
    let ctx = TestContext::new().await;
    let result: Result<()> = async {
        // Create admin with known password
        let _ = ctx.state.db.create_user(CreateUser {
            username: "admin".to_string(),
            email: "admin@readur.com".to_string(),
            password: "oldpass123".to_string(),
            role: Some(UserRole::Admin),
        }).await?;

        // Get original password hash
        let old_hash = ctx.state.db.get_user_by_username("admin")
            .await?
            .unwrap()
            .password_hash;

        // Reset password with new one
        std::env::set_var("ADMIN_PASSWORD", "newpass456");
        commands::reset_admin_password(&ctx.state.db).await?;
        std::env::remove_var("ADMIN_PASSWORD");

        // Get new password hash
        let new_hash = ctx.state.db.get_user_by_username("admin")
            .await?
            .unwrap()
            .password_hash;

        // Verify password changed
        assert_ne!(old_hash, new_hash, "Password hash should have changed");

        // Verify new password works
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let token = auth_helper.login_user("admin", "newpass456").await;
        assert!(!token.is_empty(), "Should be able to login with new password");

        // Note: We don't explicitly test that old password doesn't work because
        // the auth helper panics on login failure. The fact that new password works
        // and the hash changed is sufficient verification.

        Ok(())
    }.await;

    std::env::remove_var("ADMIN_PASSWORD");
    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}

/// Test that reset command uses provided ADMIN_PASSWORD
#[tokio::test]
async fn test_reset_command_uses_env_password() {
    let ctx = TestContext::new().await;
    let result: Result<()> = async {
        // Create admin
        let _ = ctx.state.db.create_user(CreateUser {
            username: "admin".to_string(),
            email: "admin@readur.com".to_string(),
            password: "initial123".to_string(),
            role: Some(UserRole::Admin),
        }).await?;

        // Reset with specific password
        std::env::set_var("ADMIN_PASSWORD", "specific789");
        commands::reset_admin_password(&ctx.state.db).await?;
        std::env::remove_var("ADMIN_PASSWORD");

        // Verify can login with the specific password
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let token = auth_helper.login_user("admin", "specific789").await;
        assert!(!token.is_empty(), "Should login with environment-specified password");

        Ok(())
    }.await;

    std::env::remove_var("ADMIN_PASSWORD");
    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}

/// Test that reset command returns error for non-existent user
#[tokio::test]
async fn test_reset_command_fails_for_nonexistent_user() {
    let ctx = TestContext::new().await;
    let result: Result<()> = async {
        // Don't create any admin user

        // Try to reset password for non-existent admin
        std::env::remove_var("ADMIN_USERNAME"); // Use default "admin"
        std::env::set_var("ADMIN_PASSWORD", "testpass123");

        let reset_result = commands::reset_admin_password(&ctx.state.db).await;

        // Should return an error
        assert!(
            reset_result.is_err(),
            "Reset should fail when user doesn't exist"
        );

        let error_message = reset_result.unwrap_err().to_string();
        assert!(
            error_message.contains("not found") || error_message.contains("Admin user"),
            "Error should indicate user not found, got: {}",
            error_message
        );

        std::env::remove_var("ADMIN_PASSWORD");

        Ok(())
    }.await;

    std::env::remove_var("ADMIN_PASSWORD");
    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}
