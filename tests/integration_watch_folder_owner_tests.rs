//! The global watch folder's files belong to the configured admin account
//! (ADMIN_USERNAME), never to whichever account happens to be named "admin",
//! and are skipped -- not panicked on, not misassigned -- when that admin
//! cannot be resolved.

use std::path::{Path, PathBuf};

use tempfile::TempDir;
use uuid::Uuid;

use readur::{
    models::{CreateUser, UserRole},
    scheduling::{
        watch_owner::WatchFolderOwner,
        watcher::{process_file, WatchContext},
    },
    test_utils::TestContext,
};

async fn create_user(ctx: &TestContext, username: &str, role: UserRole) -> Uuid {
    ctx.state
        .db
        .create_user(CreateUser {
            username: username.to_string(),
            email: format!("{username}@example.com"),
            password: "password123".to_string(),
            role: Some(role),
        })
        .await
        .expect("create user")
        .id
}

/// A watcher context whose global watch folder is `watch_dir`.
fn watch_context(ctx: &TestContext, watch_dir: &Path, owner: WatchFolderOwner) -> WatchContext {
    let mut config = ctx.state.config.clone();
    config.watch_folder = watch_dir.to_string_lossy().to_string();
    config.enable_per_user_watch = false;
    WatchContext {
        config,
        db: ctx.state.db.clone(),
        file_service: ctx.state.file_service.clone(),
        queue_service: (*ctx.state.queue_service).clone(),
        user_watch_manager: None,
        owner,
    }
}

fn write_watch_file(watch_dir: &Path, name: &str) -> PathBuf {
    let path = watch_dir.join(name);
    std::fs::write(&path, format!("watch folder content of {name}")).unwrap();
    path
}

async fn document_count(ctx: &TestContext) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM documents")
        .fetch_one(ctx.state.db.get_pool())
        .await
        .unwrap()
}

async fn documents_of(ctx: &TestContext, user_id: Uuid) -> Vec<String> {
    ctx.state
        .db
        .get_documents_by_user(user_id, 100, 0)
        .await
        .unwrap()
        .into_iter()
        .map(|d| d.original_filename)
        .collect()
}

#[tokio::test]
async fn global_watch_file_belongs_to_the_configured_admin_not_a_user_named_admin() {
    let ctx = TestContext::new().await;
    let admin_id = create_user(&ctx, "ops-root", UserRole::Admin).await;
    // An ordinary account that took the default admin name must get nothing.
    let decoy_id = create_user(&ctx, "admin", UserRole::User).await;
    let watch_dir = TempDir::new().unwrap();
    let wctx = watch_context(&ctx, watch_dir.path(), WatchFolderOwner::named("ops-root"));

    let path = write_watch_file(watch_dir.path(), "owned.txt");
    process_file(&path, &wctx).await.unwrap();

    assert_eq!(
        documents_of(&ctx, admin_id).await,
        vec!["owned.txt".to_string()]
    );
    assert!(documents_of(&ctx, decoy_id).await.is_empty());
}

#[tokio::test]
async fn global_watch_file_is_skipped_when_the_owner_does_not_exist() {
    let ctx = TestContext::new().await;
    create_user(&ctx, "admin", UserRole::Admin).await;
    let watch_dir = TempDir::new().unwrap();
    let wctx = watch_context(
        &ctx,
        watch_dir.path(),
        WatchFolderOwner::named("missing-admin"),
    );

    let path = write_watch_file(watch_dir.path(), "orphan.txt");
    // Skipped with Ok: no panic, no error, and no fallback to "admin".
    process_file(&path, &wctx).await.unwrap();

    assert_eq!(document_count(&ctx).await, 0);
}

#[tokio::test]
async fn global_watch_file_is_skipped_when_the_owner_is_not_an_admin() {
    let ctx = TestContext::new().await;
    create_user(&ctx, "plain-user", UserRole::User).await;
    let watch_dir = TempDir::new().unwrap();
    let wctx = watch_context(
        &ctx,
        watch_dir.path(),
        WatchFolderOwner::named("plain-user"),
    );

    let path = write_watch_file(watch_dir.path(), "not-for-users.txt");
    process_file(&path, &wctx).await.unwrap();

    assert_eq!(document_count(&ctx).await, 0);
}

/// The watcher takes its owner from ADMIN_USERNAME, the same variable the
/// seed uses for the admin it creates. This is the only test in the binary
/// that touches ADMIN_USERNAME; std serialises its own env reads and writes.
#[tokio::test]
async fn watcher_owner_follows_admin_username() {
    let ctx = TestContext::new().await;
    let admin_id = create_user(&ctx, "renamed-admin", UserRole::Admin).await;
    let decoy_id = create_user(&ctx, "admin", UserRole::Admin).await;

    let previous = std::env::var("ADMIN_USERNAME").ok();
    std::env::set_var("ADMIN_USERNAME", "renamed-admin");
    let owner = WatchFolderOwner::from_env();
    match previous {
        Some(v) => std::env::set_var("ADMIN_USERNAME", v),
        None => std::env::remove_var("ADMIN_USERNAME"),
    }
    assert_eq!(owner.username(), "renamed-admin");
    assert_eq!(
        readur::seed::admin_username_from(Some("renamed-admin".into())),
        owner.username()
    );

    let watch_dir = TempDir::new().unwrap();
    let wctx = watch_context(&ctx, watch_dir.path(), owner);
    let path = write_watch_file(watch_dir.path(), "renamed.txt");
    process_file(&path, &wctx).await.unwrap();

    assert_eq!(
        documents_of(&ctx, admin_id).await,
        vec!["renamed.txt".to_string()]
    );
    assert!(documents_of(&ctx, decoy_id).await.is_empty());
}
