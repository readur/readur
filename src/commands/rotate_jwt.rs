use anyhow::Result;

use crate::db::Database;

/// Replace the JWT signing key stored in the database with a new random one.
///
/// Every session signed with the previous key stops working once the servers
/// are restarted and load the new key. When `JWT_SECRET` is set it takes
/// precedence and the stored key is not used, which is reported as a warning.
pub async fn rotate_jwt_secret(db: &Database, jwt_secret_env_set: bool) -> Result<()> {
    if jwt_secret_env_set {
        println!("⚠️  JWT_SECRET is set; it takes precedence and the key stored in the database is not used.");
        println!("   To rotate it, change JWT_SECRET and restart the server.");
    }

    crate::jwt_signing_key::rotate(db).await?;

    println!();
    println!("✅ A new JWT signing key was stored in the database.");
    println!("   Restart every running Readur server to load it. All existing sessions");
    println!("   are then invalid and users have to sign in again.");
    Ok(())
}
