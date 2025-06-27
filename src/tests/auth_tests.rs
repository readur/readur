#[cfg(test)]
mod tests {
    use crate::auth::{create_jwt, verify_jwt};
    use crate::models::User;
    use chrono::Utc;
    use uuid::Uuid;

    fn create_test_user() -> User {
        User {
            id: Uuid::new_v4(),
            username: "testuser".to_string(),
            email: "test@example.com".to_string(),
            password_hash: Some("hashed_password".to_string()),
            role: crate::models::UserRole::User,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            oidc_subject: None,
            oidc_issuer: None,
            oidc_email: None,
            auth_provider: crate::models::AuthProvider::Local,
        }
    }

    #[test]
    fn test_create_jwt() {
        let user = create_test_user();
        let secret = "test_secret";
        
        let result = create_jwt(&user, secret);
        assert!(result.is_ok());
        
        let token = result.unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_verify_jwt_valid() {
        let user = create_test_user();
        let secret = "test_secret";
        
        let token = create_jwt(&user, secret).unwrap();
        let result = verify_jwt(&token, secret);
        
        assert!(result.is_ok());
        
        let claims = result.unwrap();
        assert_eq!(claims.sub, user.id);
        assert_eq!(claims.username, user.username);
    }

    #[test]
    fn test_verify_jwt_invalid_secret() {
        let user = create_test_user();
        let secret = "test_secret";
        let wrong_secret = "wrong_secret";
        
        let token = create_jwt(&user, secret).unwrap();
        let result = verify_jwt(&token, wrong_secret);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_jwt_malformed_token() {
        let secret = "test_secret";
        let malformed_token = "invalid.token.here";
        
        let result = verify_jwt(malformed_token, secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_verify_jwt_empty_token() {
        let secret = "test_secret";
        let empty_token = "";
        
        let result = verify_jwt(empty_token, secret);
        assert!(result.is_err());
    }
}