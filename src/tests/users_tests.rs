#[cfg(test)]
mod tests {
    use crate::models::{CreateUser, UpdateUser, UserResponse};
    use super::super::helpers::{create_test_app, create_test_user, create_admin_user, login_user};
    use axum::http::StatusCode;
    use serde_json::json;
    use tower::util::ServiceExt;

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
}