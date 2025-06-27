#[cfg(test)]
mod tests {
    use crate::models::{CreateUser, UpdateUser, UserResponse, AuthProvider, UserRole};
    use super::super::helpers::{create_test_app, create_test_user, create_admin_user, login_user};
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::util::ServiceExt;
    use uuid;

    #[tokio::test]
    async fn test_list_users() {
        let (app, _container) = create_test_app().await;
        let admin = create_admin_user(&app).await;
        let token = login_user(&app, &admin.username, "adminpass123").await;

        // Create another user
        let user2_data = json!({
            "username": "testuser2",
            "email": "test2@example.com",
            "password": "password456"
        });
        
        app.clone()
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

        let response = app
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

        assert_eq!(users.len(), 2);
        assert!(users.iter().any(|u| u.username == "adminuser"));
        assert!(users.iter().any(|u| u.username == "testuser2"));
    }

    #[tokio::test]
    async fn test_get_user_by_id() {
        let (app, _container) = create_test_app().await;
        let admin = create_admin_user(&app).await;
        let token = login_user(&app, &admin.username, "adminpass123").await;

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("GET")
                    .uri(format!("/api/users/{}", admin.id))
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

        assert_eq!(fetched_user.id, admin.id);
        assert_eq!(fetched_user.username, admin.username);
        assert_eq!(fetched_user.email, admin.email);
    }

    #[tokio::test]
    async fn test_create_user_via_api() {
        let (app, _container) = create_test_app().await;
        let admin = create_admin_user(&app).await;
        let token = login_user(&app, &admin.username, "adminpass123").await;

        let new_user_data = CreateUser {
            username: "newuser".to_string(),
            email: "new@example.com".to_string(),
            password: "newpassword".to_string(),
            role: Some(crate::models::UserRole::User),
        };

        let response = app
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

        assert_eq!(created_user.username, "newuser");
        assert_eq!(created_user.email, "new@example.com");
    }

    #[tokio::test]
    async fn test_update_user() {
        let (app, _container) = create_test_app().await;
        let admin = create_admin_user(&app).await;
        let token = login_user(&app, &admin.username, "adminpass123").await;
        
        // Create a regular user to update
        let user = create_test_user(&app).await;

        let update_data = UpdateUser {
            username: Some("updateduser".to_string()),
            email: Some("updated@example.com".to_string()),
            password: None,
        };

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri(format!("/api/users/{}", user.id))
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

        assert_eq!(updated_user.username, "updateduser");
        assert_eq!(updated_user.email, "updated@example.com");
    }

    #[tokio::test]
    async fn test_update_user_password() {
        let (app, _container) = create_test_app().await;
        let admin = create_admin_user(&app).await;
        let token = login_user(&app, &admin.username, "adminpass123").await;
        
        // Create a regular user to update
        let user = create_test_user(&app).await;

        let update_data = UpdateUser {
            username: None,
            email: None,
            password: Some("newpassword456".to_string()),
        };

        let response = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("PUT")
                    .uri(format!("/api/users/{}", user.id))
                    .header("Authorization", format!("Bearer {}", token))
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(serde_json::to_vec(&update_data).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify new password works
        let new_token = login_user(&app, "testuser", "newpassword456").await;
        assert!(!new_token.is_empty());
    }

    #[tokio::test]
    async fn test_delete_user() {
        let (app, _container) = create_test_app().await;
        let admin = create_admin_user(&app).await;
        let token = login_user(&app, &admin.username, "adminpass123").await;

        // Create another user to delete
        let user2_data = json!({
            "username": "deleteuser",
            "email": "delete@example.com",
            "password": "password456"
        });
        
        let response = app
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
        let response = app
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
        let response = app
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
        let (app, _container) = create_test_app().await;
        let admin = create_admin_user(&app).await;
        let token = login_user(&app, &admin.username, "adminpass123").await;

        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .method("DELETE")
                    .uri(format!("/api/users/{}", admin.id))
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
        let (app, _container) = create_test_app().await;

        let response = app
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
        let (_app, container) = create_test_app().await;
        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let database_url = format!("postgresql://test:test@localhost:{}/test", port);
        let db = crate::db::Database::new(&database_url).await.unwrap();

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
        let (_app, container) = create_test_app().await;
        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let database_url = format!("postgresql://test:test@localhost:{}/test", port);
        let db = crate::db::Database::new(&database_url).await.unwrap();

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
        let (_app, container) = create_test_app().await;
        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let database_url = format!("postgresql://test:test@localhost:{}/test", port);
        let db = crate::db::Database::new(&database_url).await.unwrap();

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
        let (_app, container) = create_test_app().await;
        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let database_url = format!("postgresql://test:test@localhost:{}/test", port);
        let db = crate::db::Database::new(&database_url).await.unwrap();

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
        let (app, container) = create_test_app().await;
        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let database_url = format!("postgresql://test:test@localhost:{}/test", port);
        let db = crate::db::Database::new(&database_url).await.unwrap();

        // Create regular local user
        let create_user = CreateUser {
            username: "localuser".to_string(),
            email: "local@example.com".to_string(),
            password: "password123".to_string(),
            role: Some(UserRole::User),
        };

        let user = db.create_user(create_user).await.unwrap();
        
        assert_eq!(user.auth_provider, AuthProvider::Local);
        assert!(user.password_hash.is_some());
        assert!(user.oidc_subject.is_none());

        // Test login still works
        let login_data = json!({
            "username": "localuser",
            "password": "password123"
        });

        let response = app
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