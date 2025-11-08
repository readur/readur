use anyhow::Result;
use tracing::info;
use std::env;
use crate::db::Database;
use crate::models::CreateUser;
use crate::utils::security::generate_secure_password;

pub async fn seed_admin_user(db: &Database) -> Result<()> {
    // Get admin username from env var or use default
    let admin_username = env::var("ADMIN_USERNAME").unwrap_or_else(|_| "admin".to_string());

    // Generate default email based on username
    let admin_email = format!("{}@readur.com", admin_username);

    // Check if admin user already exists
    match db.get_user_by_username(&admin_username).await {
        Ok(Some(_)) => {
            info!("✅ Admin user '{}' already exists", admin_username);
            info!("🚀 You can login at http://localhost:8000");
            info!("💡 To reset the admin password, run: readur reset-admin-password");
            return Ok(());
        }
        Ok(None) => {
            // User doesn't exist, create it
        }
        Err(e) => {
            info!("⚠️  Error checking for admin user: {}", e);
        }
    }

    // Get password from env var or generate a secure one
    let admin_password = if let Ok(pwd) = env::var("ADMIN_PASSWORD") {
        if pwd.len() < 8 {
            anyhow::bail!("ADMIN_PASSWORD must be at least 8 characters long");
        }
        pwd
    } else {
        generate_secure_password(24)
    };

    let create_user = CreateUser {
        username: admin_username.clone(),
        email: admin_email.clone(),
        password: admin_password.clone(),
        role: Some(crate::models::UserRole::Admin),
    };

    match db.create_user(create_user).await {
        Ok(user) => {
            println!();
            println!("==============================================");
            println!("  READUR ADMIN USER CREATED");
            println!("==============================================");
            println!();
            println!("Username: {}", admin_username);
            println!("Email:    {}", admin_email);
            println!("Password: {}", admin_password);
            println!("User ID:  {}", user.id);
            println!();
            println!("⚠️  SAVE THESE CREDENTIALS IMMEDIATELY!");
            println!("⚠️  This password will not be shown again.");
            println!();
            println!("==============================================");
            println!();
            info!("🚀 You can now login at http://localhost:8000");
            info!("💡 To reset the admin password later, run: readur reset-admin-password");
        }
        Err(e) => {
            info!("❌ Failed to create admin user: {}", e);
        }
    }

    Ok(())
}

