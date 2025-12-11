use anyhow::{Context, Result};
use std::env;

use crate::db::Database;
use crate::utils::security::generate_secure_password;

/// Reset the admin user's password
///
/// This command resets the admin user's password to either:
/// 1. A value specified via the ADMIN_PASSWORD environment variable, or
/// 2. A newly generated secure random password (24 characters)
///
/// The admin username defaults to "admin" but can be customized via ADMIN_USERNAME env var.
///
/// # Arguments
/// * `db` - Database connection
///
/// # Returns
/// Result indicating success or failure
pub async fn reset_admin_password(db: &Database) -> Result<()> {
    // Get admin username from env var or use default
    let admin_username = env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());

    // Check if admin user exists
    let admin_user = db
        .get_user_by_username(&admin_username)
        .await
        .context("Failed to query database for admin user")?;

    if admin_user.is_none() {
        anyhow::bail!(
            "Admin user '{}' not found. Please ensure the user exists before resetting password.",
            admin_username
        );
    }

    // Get new password from env var or generate one
    let new_password = if let Ok(pwd) = env::var("ADMIN_PASSWORD") {
        if pwd.len() < 8 {
            anyhow::bail!("ADMIN_PASSWORD must be at least 8 characters long");
        }
        pwd
    } else {
        generate_secure_password(24)
    };

    // Reset the password
    db.reset_user_password(&admin_username, &new_password)
        .await
        .context("Failed to reset admin password")?;

    // Display success message with credentials
    print_success_message(&admin_username, &new_password);

    Ok(())
}

fn print_success_message(username: &str, password: &str) {
    println!();
    println!("==============================================");
    println!("  ADMIN PASSWORD RESET SUCCESSFUL");
    println!("==============================================");
    println!();
    println!("Username: {}", username);
    println!("Password: {}", password);
    println!();
    println!("⚠️  SAVE THESE CREDENTIALS IMMEDIATELY!");
    println!("⚠️  This password will not be shown again.");
    println!();
    println!("==============================================");
    println!();
}
