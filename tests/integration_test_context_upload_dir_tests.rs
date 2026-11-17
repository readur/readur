//! `TestContext` stores uploads in a per-context directory outside the repository and removes
//! it when the context drops, so test runs leave no files in the working tree (they used to
//! accumulate in ./test-uploads). An explicit `with_upload_path` is used as given and left alone.

use std::path::{Path, PathBuf};

use readur::test_utils::{TestConfigBuilder, TestContext};
use uuid::Uuid;

async fn store_a_file(ctx: &TestContext) -> PathBuf {
    let path = ctx
        .state()
        .file_service
        .save_document_file(Uuid::new_v4(), Uuid::new_v4(), "note.txt", b"hello")
        .await
        .expect("saving a file");
    PathBuf::from(path)
}

#[tokio::test]
async fn default_upload_dir_is_private_outside_the_repo_and_removed_on_drop() {
    let first = TestContext::new().await;
    let second = TestContext::new().await;
    let dir = PathBuf::from(&first.state().config.upload_path);
    assert_ne!(
        dir,
        PathBuf::from(&second.state().config.upload_path),
        "each context gets its own directory"
    );
    assert!(dir.is_absolute(), "{}", dir.display());
    let repo = std::env::current_dir().unwrap();
    assert!(
        !dir.starts_with(&repo),
        "{} is inside the repository",
        dir.display()
    );

    let stored = store_a_file(&first).await;
    assert!(
        stored.starts_with(&dir),
        "{} not under {}",
        stored.display(),
        dir.display()
    );
    assert!(stored.exists());

    drop(first);
    assert!(!dir.exists(), "{} survived the context", dir.display());
    drop(second);
}

#[tokio::test]
async fn explicit_upload_path_is_used_and_kept() {
    let explicit = tempfile::tempdir().unwrap();
    let ctx = TestContext::with_config(
        TestConfigBuilder::default().with_upload_path(explicit.path().to_str().unwrap()),
    )
    .await;
    assert_eq!(Path::new(&ctx.state().config.upload_path), explicit.path());
    let stored = store_a_file(&ctx).await;
    drop(ctx);
    assert!(
        stored.exists(),
        "a caller-owned upload directory is not removed"
    );
}
