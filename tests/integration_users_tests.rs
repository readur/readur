#[cfg(test)]
mod tests {
    use anyhow::Result;
    use readur::models::{CreateUser, UpdateUser, UserResponse, AuthProvider, UserRole};
    use readur::test_utils::{TestContext, TestAuthHelper};
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::util::ServiceExt;
    use uuid;

    #[tokio::test]
    async fn test_list_users() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            // Create admin user using TestAuthHelper for unique credentials
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let admin = auth_helper.create_admin_user().await;
            let token = auth_helper.login_user(&admin.username, "adminpass123").await;

            // Create another user using TestAuthHelper for unique credentials
            let user2 = auth_helper.create_test_user().await;

            let response = ctx.app.clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("GET")
                        .uri("/api/users")
                        .header("Authorization", format!("Bearer {}", token))
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);

            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let users: Vec<UserResponse> = serde_json::from_slice(&body).unwrap();

            // Ensure we have at least our 2 created users
            assert!(users.len() >= 2);
            assert!(users.iter().any(|u| u.username == admin.username));
            assert!(users.iter().any(|u| u.username == user2.username));
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_get_user_by_id() {
        let ctx = TestContext::new().await;
        
        // Ensure cleanup happens even if test fails
        let result: Result<()> = async {
            let auth_helper = TestAuthHelper::new(ctx.app.clone());
            let admin = auth_helper.create_admin_user().await;
            let token = auth_helper.login_user(&admin.username, "adminpass123").await;

            let response = ctx.app.clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("GET")
                        .uri(format!("/api/users/{}", admin.id()))
                        .header("Authorization", format!("Bearer {}", token))
                        .body(axum::body::Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK);

            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            let fetched_user: UserResponse = serde_json::from_slice(&body).unwrap();

            assert_eq!(fetched_user.id.to_string(), admin.id());
            assert_eq!(fetched_user.username, admin.username);
            assert_eq!(fetched_user.email, admin.user_response.email);
            
            Ok(())
        }.await;
        
        // Always cleanup database connections and test data
        if let Err(e) = ctx.cleanup_and_close().await {
            eprintln!("Warning: Test cleanup failed: {}", e);
        }
        
        result.unwrap();
    }

    #[tokio::test]
    async fn test_create_user_via_api() {
        let ctx = TestContext::new().await;
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let admin = auth_helper.create_admin_user().await;
        let token = auth_helper.login_user(&admin.username, "adminpass123").await;

        let unique_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("newuser_{}", unique_suffix);
        let email = format!("new_{}@example.com", unique_suffix);
        
        let new_user_data = CreateUser {
            username: username.clone(),
            email: email.clone(),
            password: "newpassword".to_string(),
            role: Some(readur::models::UserRole::User),
        };

        let response = ctx.app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/users")
                    .header("Authorization", format!("Bearer {}", token))
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_vec(&new_user_data).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_user: UserResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(created_user.username, username);
        assert_eq!(created_user.email, email);
    }

    #[tokio::test]
    async fn test_update_user() {
        let ctx = TestContext::new().await;
        
        // Create admin user using TestAuthHelper for unique credentials
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let admin = auth_helper.create_admin_user().await;
        let token = auth_helper.login_user(&admin.username, "adminpass123").await;
        
        // Create a regular user using TestAuthHelper for unique credentials
        let user = auth_helper.create_test_user().await;

        let unique_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let updated_username = format!("updateduser_{}", unique_suffix);
        let updated_email = format!("updated_{}@example.com", unique_suffix);
        
        let update_data = UpdateUser {
            username: Some(updated_username.clone()),
            email: Some(updated_email.clone()),
            password: None,
        };

        let response = ctx.app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri(format!("/api/users/{}", user.user_response.id))
                    .header("Authorization", format!("Bearer {}", token))
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_vec(&update_data).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let updated_user: UserResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(updated_user.username, updated_username);
        assert_eq!(updated_user.email, updated_email);
    }

    #[tokio::test]
    async fn test_update_user_password() {
        let ctx = TestContext::new().await;
        
        // Create admin user using TestAuthHelper for unique credentials
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let admin = auth_helper.create_admin_user().await;
        let token = auth_helper.login_user(&admin.username, "adminpass123").await;
        
        // Create a regular user using TestAuthHelper for unique credentials
        let user = auth_helper.create_test_user().await;

        let update_data = UpdateUser {
            username: None,
            email: None,
            password: Some("newpassword456".to_string()),
        };

        let response = ctx.app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri(format!("/api/users/{}", user.user_response.id))
                    .header("Authorization", format!("Bearer {}", token))
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_vec(&update_data).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify new password works
        let new_token = auth_helper.login_user(&user.username, "newpassword456").await;
        assert!(!new_token.is_empty());
    }

    #[tokio::test]
    async fn test_delete_user() {
        let ctx = TestContext::new().await;
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let admin = auth_helper.create_admin_user().await;
        let token = auth_helper.login_user(&admin.username, "adminpass123").await;

        // Create another user to delete
        let user2_data = json!({
            "username": "deleteuser",
            "email": "delete@example.com",
            "password": "password456"
        });
        
        let response = ctx.app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/auth/register")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_vec(&user2_data).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let user2: UserResponse = serde_json::from_slice(&body).unwrap();

        // Delete the user
        let response = ctx.app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/users/{}", user2.id))
                    .header("Authorization", format!("Bearer {}", token))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        // Verify user is deleted
        let response = ctx.app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri(format!("/api/users/{}", user2.id))
                    .header("Authorization", format!("Bearer {}", token))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_cannot_delete_self() {
        let ctx = TestContext::new().await;
        let auth_helper = TestAuthHelper::new(ctx.app.clone());
        let admin = auth_helper.create_admin_user().await;
        let token = auth_helper.login_user(&admin.username, "adminpass123").await;

        let response = ctx.app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/users/{}", admin.id()))
                    .header("Authorization", format!("Bearer {}", token))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_users_require_auth() {
        let ctx = TestContext::new().await;

        let response = ctx.app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri("/api/users")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // OIDC Database Tests
    #[tokio::test]
    async fn test_create_oidc_user() {
        let ctx = TestContext::new().await;
        let db = &ctx.state.db;

        // Generate random identifiers to avoid test interference
        let test_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let test_username = format!("oidcuser_{}", test_id);
        let test_email = format!("oidc_{}@example.com", test_id);
        let test_subject = format!("oidc-subject-{}", test_id);

        let create_user = CreateUser {
            username: test_username.clone(),
            email: test_email.clone(),
            password: "".to_string(), // Not used for OIDC
            role: Some(UserRole::User),
        };

        let user = db.create_oidc_user(
            create_user,
            &test_subject,
            "https://provider.example.com",
            &test_email,
        ).await.unwrap();

        assert_eq!(user.username, test_username);
        assert_eq!(user.email, test_email);
        assert_eq!(user.oidc_subject, Some(test_subject));
        assert_eq!(user.oidc_issuer, Some("https://provider.example.com".to_string()));
        assert_eq!(user.oidc_email, Some(test_email.clone()));
        assert_eq!(user.auth_provider, AuthProvider::Oidc);
        assert!(user.password_hash.is_none());
    }

    #[tokio::test]
    async fn test_get_user_by_oidc_subject() {
        let ctx = TestContext::new().await;
        let db = &ctx.state.db;

        // Generate random identifiers to avoid test interference
        let test_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let test_username = format!("oidcuser_{}", test_id);
        let test_email = format!("oidc_{}@example.com", test_id);
        let test_subject = format!("oidc-subject-{}", test_id);

        // Create OIDC user
        let create_user = CreateUser {
            username: test_username,
            email: test_email.clone(),
            password: "".to_string(),
            role: Some(UserRole::User),
        };

        let created_user = db.create_oidc_user(
            create_user,
            &test_subject,
            "https://provider.example.com",
            &test_email,
        ).await.unwrap();

        // Retrieve by OIDC subject
        let found_user = db.get_user_by_oidc_subject(
            &test_subject,
            "https://provider.example.com"
        ).await.unwrap();

        assert!(found_user.is_some());
        let user = found_user.unwrap();
        assert_eq!(user.id, created_user.id);
        assert_eq!(user.oidc_subject, Some(test_subject));
    }

    #[tokio::test]
    async fn test_get_user_by_oidc_subject_not_found() {
        let ctx = TestContext::new().await;
        let db = &ctx.state.db;

        // Generate random subject that definitely doesn't exist
        let test_id = uuid::Uuid::new_v4().to_string();
        let nonexistent_subject = format!("nonexistent-subject-{}", test_id);
        
        let found_user = db.get_user_by_oidc_subject(
            &nonexistent_subject,
            "https://provider.example.com"
        ).await.unwrap();

        assert!(found_user.is_none());
    }

    #[tokio::test]
    async fn test_oidc_user_different_issuer() {
        let ctx = TestContext::new().await;
        let db = &ctx.state.db;

        // Generate random identifiers to avoid test interference
        let test_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let test_username = format!("oidcuser_{}", test_id);
        let test_email = format!("oidc_{}@example.com", test_id);
        let test_subject = format!("same-subject-{}", test_id);

        // Create OIDC user with one issuer
        let create_user = CreateUser {
            username: test_username,
            email: test_email.clone(),
            password: "".to_string(),
            role: Some(UserRole::User),
        };

        db.create_oidc_user(
            create_user,
            &test_subject,
            "https://provider1.example.com",
            &test_email,
        ).await.unwrap();

        // Try to find with different issuer (should not find)
        let found_user = db.get_user_by_oidc_subject(
            &test_subject,
            "https://provider2.example.com"
        ).await.unwrap();

        assert!(found_user.is_none());
    }

    #[tokio::test]
    async fn test_local_user_login_works() {
        let ctx = TestContext::new().await;
        let db = &ctx.state.db;

        // Create regular local user with unique credentials
        let unique_suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let username = format!("localuser_{}", unique_suffix);
        let email = format!("local_{}@example.com", unique_suffix);
        
        let create_user = CreateUser {
            username: username.clone(),
            email: email.clone(),
            password: "password123".to_string(),
            role: Some(UserRole::User),
        };

        let user = db.create_user(create_user).await.unwrap();
        
        assert_eq!(user.auth_provider, AuthProvider::Local);
        assert!(user.password_hash.is_some());
        assert!(user.oidc_subject.is_none());

        // Test login still works
        let login_data = json!({
            "username": username,
            "password": "password123"
        });

        let response = ctx.app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_vec(&login_data).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}