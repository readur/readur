//! The local storage backend deletes a document's file at the path recorded
//! for it, and every thumbnail / OCR review image derived from it.

use readur::storage::local::LocalStorageBackend;
use readur::storage::StorageBackend;
use std::fs;
use uuid::Uuid;

fn backend() -> (LocalStorageBackend, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    for sub in ["documents", "thumbnails", "processed_images"] {
        fs::create_dir_all(dir.path().join(sub)).unwrap();
    }
    (
        LocalStorageBackend::new(dir.path().to_string_lossy().to_string()),
        dir,
    )
}

/// The recorded file is deleted even when its name is not `<id>.<ext>`, and
/// a different file that happens to carry the document's filename survives
/// (deletion used to guess `documents/<filename>`).
#[tokio::test]
async fn deletes_the_recorded_file_and_never_a_file_named_like_the_document() {
    let (backend, dir) = backend();
    let doc = Uuid::new_v4();
    let recorded = dir
        .path()
        .join("documents")
        .join(format!("{}.pdf", Uuid::new_v4()));
    let namesake = dir.path().join("documents").join("invoice.pdf");
    fs::write(&recorded, b"mine").unwrap();
    fs::write(&namesake, b"someone else's").unwrap();

    backend
        .delete_document_files(Uuid::new_v4(), doc, &recorded.to_string_lossy())
        .await
        .unwrap();

    assert!(!recorded.exists());
    assert!(
        namesake.exists(),
        "a file merely named like the document must not be deleted"
    );
}

#[tokio::test]
async fn deletes_every_derived_file_name() {
    let (backend, dir) = backend();
    let doc = Uuid::new_v4();
    let stem = Uuid::new_v4();
    let recorded = dir.path().join("documents").join(format!("{stem}.png"));
    let thumbs = dir.path().join("thumbnails");
    let processed = dir.path().join("processed_images");
    let derived = [
        thumbs.join(format!("{doc}_thumb.jpg")),
        thumbs.join(format!("{stem}_thumb.jpg")),
        processed.join(format!("{doc}_processed.png")),
        processed.join(format!("{doc}_processed_1700000000.jpg")),
    ];
    let other = processed.join(format!("{}_processed_1700000000.jpg", Uuid::new_v4()));
    fs::write(&recorded, b"x").unwrap();
    for f in derived.iter().chain([&other]) {
        fs::write(f, b"x").unwrap();
    }

    backend
        .delete_document_files(Uuid::new_v4(), doc, &recorded.to_string_lossy())
        .await
        .unwrap();

    assert!(!recorded.exists());
    for f in &derived {
        assert!(!f.exists(), "{} should be deleted", f.display());
    }
    assert!(
        other.exists(),
        "another document's review image is untouched"
    );
}

#[tokio::test]
async fn a_missing_file_is_already_deleted() {
    let (backend, dir) = backend();
    let gone = dir.path().join("documents").join("gone.pdf");
    backend
        .delete_document_files(Uuid::new_v4(), Uuid::new_v4(), &gone.to_string_lossy())
        .await
        .unwrap();
}

#[tokio::test]
async fn refuses_to_delete_a_recorded_path_outside_the_upload_directory() {
    let (backend, _dir) = backend();
    let outside = tempfile::NamedTempFile::new().unwrap();

    let err = backend
        .delete_document_files(
            Uuid::new_v4(),
            Uuid::new_v4(),
            &outside.path().to_string_lossy(),
        )
        .await
        .unwrap_err();

    assert!(
        err.to_string().contains("outside the upload directory"),
        "{err}"
    );
    assert!(outside.path().exists());
}

/// A recorded `s3://<key>` path resolves below the upload directory, the
/// same way `FileService::read_file` reads it.
#[tokio::test]
async fn an_s3_style_path_resolves_below_the_upload_directory() {
    let (backend, dir) = backend();
    let doc = Uuid::new_v4();
    let file = dir.path().join("documents").join(format!("{doc}.txt"));
    fs::write(&file, b"x").unwrap();

    backend
        .delete_document_files(Uuid::new_v4(), doc, &format!("s3://documents/{doc}.txt"))
        .await
        .unwrap();

    assert!(!file.exists());
}
