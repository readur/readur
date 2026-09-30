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
            let auth_helper = ctx.auth_helper();
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
            let auth_helper = ctx.auth_helper();
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
        let auth_helper = ctx.auth_helper();
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
        let auth_helper = ctx.auth_helper();
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
            is_active: None,
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
        let auth_helper = ctx.auth_helper();
        let admin = auth_helper.create_admin_user().await;
        let token = auth_helper.login_user(&admin.username, "adminpass123").await;
        
        // Create a regular user using TestAuthHelper for unique credentials
        let user = auth_helper.create_test_user().await;

        let update_data = UpdateUser {
            username: None,
            email: None,
            password: Some("newpassword456".to_string()),
            is_active: None,
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
        let auth_helper = ctx.auth_helper();
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
        let auth_helper = ctx.auth_helper();
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

    async fn login_from(ctx: &TestContext, ip: &str, username: &str, password: &str) -> StatusCode {
        let addr: std::net::SocketAddr = format!("{}:40000", ip).parse().unwrap();
        let mut request = axum::http::Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from(
                serde_json::to_vec(&json!({"username": username, "password": password})).unwrap(),
            ))
            .unwrap();
        request.extensions_mut().insert(axum::extract::ConnectInfo(addr));
        ctx.app.clone().oneshot(request).await.unwrap().status()
    }

    #[tokio::test]
    async fn test_failed_logins_from_one_client_do_not_lock_out_others() {
        let ctx = TestContext::new().await;
        let unique = uuid::Uuid::new_v4().simple().to_string();
        let username = format!("lockout_{}", &unique[..12]);
        let password = readur::test_utils::test_password();
        ctx.state
            .db
            .create_user(CreateUser {
                username: username.clone(),
                email: format!("{}@example.com", username),
                password: password.clone(),
                role: Some(UserRole::User),
            })
            .await
            .unwrap();

        for _ in 0..10 {
            assert_eq!(login_from(&ctx, "198.51.100.10", &username, &readur::test_utils::test_password()).await, StatusCode::UNAUTHORIZED);
        }
        // The failing client is now limited for this account, even with the right password...
        assert_eq!(
            login_from(&ctx, "198.51.100.10", &username.to_uppercase(), &password).await,
            StatusCode::TOO_MANY_REQUESTS
        );
        // ...while the account owner on another address can still sign in.
        assert_eq!(login_from(&ctx, "203.0.113.20", &username, &password).await, StatusCode::OK);
    }

    async fn json_request(
        ctx: &TestContext,
        method: &str,
        uri: &str,
        token: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let body = match body {
            Some(b) => axum::body::Body::from(serde_json::to_vec(&b).unwrap()),
            None => axum::body::Body::empty(),
        };
        let response = ctx.app.clone()
            .oneshot(
                axum::http::Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("Authorization", format!("Bearer {}", token))
                    .header("Content-Type", "application/json")
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null))
    }

    #[tokio::test]
    async fn test_admin_endpoints_reject_regular_users() {
        let ctx = TestContext::new().await;
        let auth_helper = ctx.auth_helper();
        let user = auth_helper.create_test_user().await;
        let token = auth_helper.login_user(&user.username, &user.password).await;

        let (status, _) = json_request(&ctx, "GET", "/api/users", &token, None).await;
        assert_eq!(status, StatusCode::FORBIDDEN);

        let (status, _) = json_request(
            &ctx,
            "PUT",
            &format!("/api/users/{}", user.id()),
            &token,
            Some(json!({ "is_active": true })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_admin_create_user_validates_fields() {
        let ctx = TestContext::new().await;
        let auth_helper = ctx.auth_helper();
        let admin = auth_helper.create_admin_user().await;
        let token = auth_helper.login_user(&admin.username, &admin.password).await;

        for body in [
            json!({ "username": "../escape", "email": "ok@example.com", "password": "password123" }),
            json!({ "username": "valid_name", "email": "not-an-email", "password": "password123" }),
            json!({ "username": "valid_name", "email": "ok@example.com", "password": "short" }),
        ] {
            let (status, _) = json_request(&ctx, "POST", "/api/users", &token, Some(body.clone())).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "expected rejection for {}", body);
        }
    }

    #[tokio::test]
    async fn test_admin_can_approve_and_deactivate_accounts() {
        let ctx = TestContext::new().await;
        let auth_helper = ctx.auth_helper();
        let admin = auth_helper.create_admin_user().await;
        let admin_token = auth_helper.login_user(&admin.username, &admin.password).await;

        // An account awaiting approval, as created by self-registration.
        let username = format!("pending_{}", &uuid::Uuid::new_v4().simple().to_string()[..10]);
        let password = readur::test_utils::test_password();
        let pending = ctx.state.db
            .create_user_with_status(
                CreateUser {
                    username: username.clone(),
                    email: format!("{}@example.com", username),
                    password: password.clone(),
                    role: Some(UserRole::User),
                },
                false,
            )
            .await
            .unwrap();

        let (status, users) = json_request(&ctx, "GET", "/api/users", &admin_token, None).await;
        assert_eq!(status, StatusCode::OK);
        let listed = users.as_array().unwrap().iter()
            .find(|u| u["username"] == username.as_str())
            .expect("pending account should be listed");
        assert_eq!(listed["is_active"], false);

        let (status, updated) = json_request(
            &ctx,
            "PUT",
            &format!("/api/users/{}", pending.id),
            &admin_token,
            Some(json!({ "is_active": true })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(updated["is_active"], true);

        // Now the account can sign in; deactivating it revokes the session.
        let user_token = auth_helper.login_user(&username, &password).await;
        let (status, _) = json_request(&ctx, "GET", "/api/auth/me", &user_token, None).await;
        assert_eq!(status, StatusCode::OK);

        let (status, _) = json_request(
            &ctx,
            "PUT",
            &format!("/api/users/{}", pending.id),
            &admin_token,
            Some(json!({ "is_active": false })),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let (status, _) = json_request(&ctx, "GET", "/api/auth/me", &user_token, None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_admin_cannot_deactivate_self_or_set_invalid_password() {
        let ctx = TestContext::new().await;
        let auth_helper = ctx.auth_helper();
        let admin = auth_helper.create_admin_user().await;
        let token = auth_helper.login_user(&admin.username, &admin.password).await;

        let (status, _) = json_request(
            &ctx,
            "PUT",
            &format!("/api/users/{}", admin.id()),
            &token,
            Some(json!({ "is_active": false })),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN);

        let (status, _) = json_request(
            &ctx,
            "PUT",
            &format!("/api/users/{}", admin.id()),
            &token,
            Some(json!({ "password": "short" })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);

        // Still active and the session still works.
        let (status, _) = json_request(&ctx, "GET", "/api/auth/me", &token, None).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_admin_can_edit_accounts_with_legacy_usernames() {
        let ctx = TestContext::new().await;
        let auth_helper = ctx.auth_helper();
        let admin = auth_helper.create_admin_user().await;
        let token = auth_helper.login_user(&admin.username, &admin.password).await;

        // A username and email that predate the current validation rules.
        let suffix = &uuid::Uuid::new_v4().simple().to_string()[..10];
        let legacy_username = format!("legacy user {}", suffix);
        let legacy_email = format!("legacy {}", suffix);
        let legacy = ctx.state.db
            .create_user(CreateUser {
                username: legacy_username.clone(),
                email: legacy_email.clone(),
                password: readur::test_utils::test_password(),
                role: Some(UserRole::User),
            })
            .await
            .unwrap();
        let uri = format!("/api/users/{}", legacy.id);

        // Sending the unchanged values back (as the edit form does) is accepted.
        let (status, updated) = json_request(
            &ctx,
            "PUT",
            &uri,
            &token,
            Some(json!({ "username": legacy_username, "email": legacy_email, "is_active": false })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{}", updated);
        assert_eq!(updated["is_active"], false);
        assert_eq!(updated["username"], legacy_username.as_str());

        // Changing either one still has to satisfy the rules.
        for body in [
            json!({ "username": "still invalid", "email": legacy_email }),
            json!({ "username": legacy_username, "email": "still invalid" }),
        ] {
            let (status, _) = json_request(&ctx, "PUT", &uri, &token, Some(body.clone())).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "expected rejection for {}", body);
        }

        // Updating a user that does not exist is a 404.
        let (status, _) = json_request(
            &ctx,
            "PUT",
            &format!("/api/users/{}", uuid::Uuid::new_v4()),
            &token,
            Some(json!({ "is_active": true })),
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}