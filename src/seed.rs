use anyhow::Result;
use tracing::info;
use crate::db::Database;
use crate::models::CreateUser;

pub async fn seed_admin_user(db: &Database) -> Result<()> {
    let admin_username = "admin";
    let admin_email = "admin@readur.com";
    let admin_password = "readur2024";

    // Check if admin user already exists
    match db.get_user_by_username(admin_username).await {
        Ok(Some(_)) => {
            info!("✅ ADMIN USER ALREADY EXISTS!");
            info!("📧 Email: {}", admin_email);
            info!("👤 Username: {}", admin_username);
            info!("🔑 Password: {}", admin_password);
            info!("🚀 You can now login to the application at http://localhost:8000");
            return Ok(());
        }
        Ok(None) => {
            // User doesn't exist, create it
        }
        Err(e) => {
            info!("Error checking for admin user: {}", e);
        }
    }

    let create_user = CreateUser {
        username: admin_username.to_string(),
        email: admin_email.to_string(),
        password: admin_password.to_string(),
        role: Some(crate::models::UserRole::Admin),
    };

    match db.create_user(create_user).await {
        Ok(user) => {
            info!("✅ ADMIN USER CREATED SUCCESSFULLY!");
            info!("📧 Email: {}", admin_email);
            info!("👤 Username: {}", admin_username);
            info!("🔑 Password: {}", admin_password);
            info!("🆔 User ID: {}", user.id);
            info!("🚀 You can now login to the application at http://localhost:8000");
        }
        Err(e) => {
            info!("Failed to create admin user: {}", e);
        }
    }

    Ok(())
}

