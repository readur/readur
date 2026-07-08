use readur::models::Settings;
use readur::ocr::enhanced::EnhancedOcrService;
use readur::services::file_service::FileService;

/// Issue #722: extract_text must handle s3:// paths by downloading via the
/// storage backend instead of failing local-path resolution.
#[tokio::test]
async fn test_extract_text_handles_s3_prefixed_path() {
    let upload_dir = tempfile::tempdir().unwrap();
    let docs_dir = upload_dir.path().join("documents");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();
    tokio::fs::write(docs_dir.join("test.txt"), b"hello from storage backend")
        .await
        .unwrap();

    let temp_dir = tempfile::tempdir().unwrap();
    // FileService::new is deprecated in favor of newer constructors, but the
    // deprecated path is exactly the legacy (local-only) behavior under test here.
    #[allow(deprecated)]
    let file_service = FileService::new(upload_dir.path().to_string_lossy().to_string());
    let ocr = EnhancedOcrService::new(
        temp_dir.path().to_string_lossy().to_string(),
        file_service,
        50,  // max_pdf_size_mb
        50,  // max_office_document_size_mb
        60,  // ocr_timeout_seconds
    );

    let settings = Settings::default();
    let result = ocr
        .extract_text("s3://documents/test.txt", "text/plain", &settings, None)
        .await
        .expect("s3:// path should be downloaded and extracted");

    assert_eq!(result.text, "hello from storage backend");

    // Temp download must be cleaned up
    let mut entries = tokio::fs::read_dir(temp_dir.path()).await.unwrap();
    assert!(entries.next_entry().await.unwrap().is_none(), "temp download not cleaned up");
}

#[cfg(feature = "ocr")]
#[tokio::test]
async fn test_thumbnail_for_s3_prefixed_path() {
    let upload_dir = tempfile::tempdir().unwrap();
    let docs_dir = upload_dir.path().join("documents");
    tokio::fs::create_dir_all(&docs_dir).await.unwrap();

    // Tiny valid PNG generated with the image crate (already a dependency)
    let img = image::RgbImage::new(4, 4);
    img.save(docs_dir.join("pic.png")).unwrap();

    // FileService::new is deprecated in favor of newer constructors, but the
    // deprecated path is exactly the legacy (local-only) behavior under test here.
    #[allow(deprecated)]
    let file_service = readur::services::file_service::FileService::new(
        upload_dir.path().to_string_lossy().to_string(),
    );

    let thumb = file_service
        .get_or_generate_thumbnail("s3://documents/pic.png", "pic.png")
        .await
        .expect("thumbnail generation should work for s3:// paths");
    assert!(!thumb.is_empty());
}
