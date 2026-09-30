//! The JWT signing key stored in the database when JWT_SECRET is not set.

use readur::jwt_signing_key::{self, SigningKeyOrigin};
use readur::test_utils::TestContext;

#[tokio::test]
async fn stored_key_is_generated_once_and_shared() {
    let ctx = TestContext::new().await;
    let db = &ctx.state().db;

    let (first, origin) = jwt_signing_key::resolve(db, "").await.unwrap();
    assert!(matches!(origin, SigningKeyOrigin::Generated | SigningKeyOrigin::Database));
    assert!(first.len() >= 64, "generated key should be long");

    // Another start (or replica) against the same database gets the same key.
    let (second, origin) = jwt_signing_key::resolve(db, "").await.unwrap();
    assert_eq!(origin, SigningKeyOrigin::Database);
    assert_eq!(first, second);

    // Concurrent first starts converge on one value.
    let (a, b) = tokio::join!(
        db.get_or_insert_server_secret("concurrency_probe", "value-a"),
        db.get_or_insert_server_secret("concurrency_probe", "value-b"),
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_eq!(a.0, b.0);
    assert!(!(a.1 && b.1), "at most one caller stores its value");

    let _ = ctx.cleanup_and_close().await;
}

#[tokio::test]
async fn jwt_secret_takes_precedence_over_the_stored_key() {
    let ctx = TestContext::new().await;
    let configured = "0f1e2d3c4b5a69788796a5b4c3d2e1f00f1e2d3c4b5a6978";
    let (key, origin) = jwt_signing_key::resolve(&ctx.state().db, configured).await.unwrap();
    assert_eq!(origin, SigningKeyOrigin::Environment);
    assert_eq!(key, configured);

    let _ = ctx.cleanup_and_close().await;
}
