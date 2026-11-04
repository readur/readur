//! `sync_webdav_directories` only prunes rows under the folder being synced, and
//! never prunes when the caller reports that discovery was incomplete.

use anyhow::Result;
use readur::{
    models::{CreateUser, CreateWebDAVDirectory, UserRole},
    test_utils::TestContext,
};
use uuid::Uuid;

fn dir(user_id: Uuid, path: &str, etag: &str) -> CreateWebDAVDirectory {
    CreateWebDAVDirectory {
        user_id,
        directory_path: path.to_string(),
        directory_etag: etag.to_string(),
        file_count: 0,
        total_size_bytes: 0,
    }
}

async fn create_user(ctx: &TestContext, suffix: &str) -> Uuid {
    let user = ctx
        .state
        .db
        .create_user(CreateUser {
            username: format!("scope_{suffix}_{}", Uuid::new_v4().simple()),
            email: format!("scope_{suffix}_{}@example.com", Uuid::new_v4().simple()),
            password: "password123".to_string(),
            role: Some(UserRole::User),
        })
        .await
        .expect("create user");
    user.id
}

async fn stored_paths(ctx: &TestContext, user_id: Uuid) -> Vec<String> {
    let mut paths: Vec<String> = ctx
        .state
        .db
        .list_webdav_directories(user_id)
        .await
        .expect("list directories")
        .into_iter()
        .map(|d| d.directory_path)
        .collect();
    paths.sort();
    paths
}

#[tokio::test]
async fn sync_of_one_folder_leaves_other_folders_untouched() {
    let ctx = TestContext::new().await;
    let result: Result<()> = async {
        let db = &ctx.state.db;
        let user_id = create_user(&ctx, "two_folders").await;

        db.bulk_create_or_update_webdav_directories(&[
            dir(user_id, "/FolderA", "a"),
            dir(user_id, "/FolderA/old", "a-old"),
            dir(user_id, "/FolderA/kept", "a-kept"),
            dir(user_id, "/FolderAB", "ab"), // shares a prefix but is a different folder
            dir(user_id, "/FolderB", "b"),
            dir(user_id, "/FolderB/sub", "b-sub"),
        ])
        .await?;

        // Full rescan of /FolderA: `old` is gone, `new` appeared.
        let (saved, deleted) = db
            .sync_webdav_directories(
                user_id,
                "/FolderA",
                &[
                    dir(user_id, "/FolderA", "a2"),
                    dir(user_id, "/FolderA/kept", "a-kept"),
                    dir(user_id, "/FolderA/new", "a-new"),
                ],
                true,
            )
            .await?;

        assert_eq!(saved.len(), 3);
        assert_eq!(deleted, 1, "only /FolderA/old should be pruned");
        assert_eq!(
            stored_paths(&ctx, user_id).await,
            vec![
                "/FolderA".to_string(),
                "/FolderA/kept".to_string(),
                "/FolderA/new".to_string(),
                "/FolderAB".to_string(),
                "/FolderB".to_string(),
                "/FolderB/sub".to_string(),
            ]
        );
        Ok(())
    }
    .await;

    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}

#[tokio::test]
async fn incomplete_discovery_never_prunes() {
    let ctx = TestContext::new().await;
    let result: Result<()> = async {
        let db = &ctx.state.db;
        let user_id = create_user(&ctx, "no_prune").await;

        db.bulk_create_or_update_webdav_directories(&[
            dir(user_id, "/Docs/ok", "ok"),
            dir(user_id, "/Docs/broken", "broken"),
            dir(user_id, "/Docs/broken/deep", "deep"),
        ])
        .await?;

        // /Docs/broken could not be listed, so only the healthy sibling is reported
        // and the caller asks for no pruning.
        let (saved, deleted) = db
            .sync_webdav_directories(user_id, "/Docs", &[dir(user_id, "/Docs/ok", "ok2")], false)
            .await?;

        assert_eq!(saved.len(), 1);
        assert_eq!(deleted, 0);
        assert_eq!(
            stored_paths(&ctx, user_id).await,
            vec![
                "/Docs/broken".to_string(),
                "/Docs/broken/deep".to_string(),
                "/Docs/ok".to_string(),
            ]
        );
        Ok(())
    }
    .await;

    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}

#[tokio::test]
async fn root_folder_scope_covers_everything_and_empty_discovery_prunes_scope() {
    let ctx = TestContext::new().await;
    let result: Result<()> = async {
        let db = &ctx.state.db;
        let user_id = create_user(&ctx, "root").await;

        db.bulk_create_or_update_webdav_directories(&[
            dir(user_id, "/", "root"),
            dir(user_id, "/A", "a"),
            dir(user_id, "/B/x", "bx"),
        ])
        .await?;

        let (_, deleted) = db.sync_webdav_directories(user_id, "/", &[], true).await?;
        assert_eq!(deleted, 3);
        assert!(stored_paths(&ctx, user_id).await.is_empty());
        Ok(())
    }
    .await;

    if let Err(e) = ctx.cleanup_and_close().await {
        eprintln!("Warning: Test cleanup failed: {}", e);
    }
    result.unwrap();
}
