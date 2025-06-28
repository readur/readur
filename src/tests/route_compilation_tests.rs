/// Tests to ensure route compilation and basic functionality
/// These tests focus on catching compilation errors in route modules

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    
    #[test]
    fn test_queue_routes_module_compiles() {
        // This test ensures the queue routes module compiles without errors
        // It would catch missing imports like the Row trait issue
        let _router = crate::routes::queue::router();
        
        // Test that required_admin function compiles
        use crate::models::{UserRole, AuthProvider};
        let test_user = crate::auth::AuthUser {
            user: crate::models::User {
                id: uuid::Uuid::new_v4(),
                username: "test".to_string(),
                email: "test@example.com".to_string(),
                password_hash: Some("hash".to_string()),
                role: UserRole::User,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                oidc_subject: None,
                oidc_issuer: None,
                oidc_email: None,
                auth_provider: AuthProvider::Local,
            },
        };
        
        // This function call would fail if there were compilation issues
        let result = crate::routes::queue::require_admin(&test_user);
        assert_eq!(result, Err(StatusCode::FORBIDDEN));
    }
    
    #[test]
    fn test_ignored_files_routes_module_compiles() {
        // This test ensures the ignored_files routes module compiles
        let _router = crate::routes::ignored_files::ignored_files_routes();
    }
    
    #[test]
    fn test_all_route_modules_compile() {
        // Test that all main route modules compile
        let _auth_router = crate::routes::auth::router();
        let _documents_router = crate::routes::documents::router();
        let _labels_router = crate::routes::labels::router();
        let _metrics_router = crate::routes::metrics::router();
        let _prometheus_router = crate::routes::prometheus_metrics::router();
        let _queue_router = crate::routes::queue::router();
        let _search_router = crate::routes::search::router();
        let _settings_router = crate::routes::settings::router();
        let _sources_router = crate::routes::sources::router();
        let _users_router = crate::routes::users::router();
        let _webdav_router = crate::routes::webdav::router();
        let _ignored_files_router = crate::routes::ignored_files::ignored_files_routes();
    }
    
    #[test]
    fn test_sql_imports_are_available() {
        // Test that required SQL traits are available
        // This would catch missing Row trait imports
        use sqlx::Row;
        
        // This is a compile-time test - if Row trait is not available, this won't compile
        let _row_method_exists = |row: &sqlx::postgres::PgRow| {
            let _: Result<i32, _> = row.try_get("test");
        };
    }
}