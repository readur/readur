//! Every extractor's scratch files leave TEMP_DIR on every way out: success,
//! and a future dropped mid-run (job cancellation, `with_ocr_timeout`).
//!
//! Like the other engine tests these need poppler-utils, ocrmypdf and
//! tesseract installed.

use super::*;
use std::future::Future;
use std::path::Path;
use std::time::Duration;

#[allow(deprecated)]
fn service(temp_dir: &Path) -> EnhancedOcrService {
    let dir = temp_dir.to_string_lossy().into_owned();
    EnhancedOcrService::new(dir.clone(), FileService::new(dir), 100, 100, 300)
}

fn fixture(path: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(path)
        .to_string_lossy()
        .into_owned()
}

fn entries(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect()
}

fn assert_empty(dir: &Path, what: &str) {
    let left = entries(dir);
    assert!(left.is_empty(), "{what} left temp files behind: {left:?}");
}

/// Polls `fut` until it has written something into `dir`, then drops it.
/// Returns false if the future finished before any file appeared.
async fn drop_once_files_appear<F: Future>(dir: &Path, fut: F) -> bool {
    drop_once_a_file_matches(dir, fut, |_| true).await
}

/// Polls `fut` until a file whose name satisfies `wanted` appears in `dir`,
/// then drops it. Returns false if the future finished first.
async fn drop_once_a_file_matches<F: Future>(
    dir: &Path,
    fut: F,
    wanted: impl Fn(&str) -> bool,
) -> bool {
    let appeared = async {
        while !entries(dir).iter().any(|name| wanted(name)) {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    };
    tokio::select! {
        _ = fut => false,
        () = appeared => true,
    }
}

/// After a drop, killed child processes need a moment to exit; nothing may
/// reappear after that.
async fn assert_empty_after_drop(dir: &Path, what: &str) {
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_empty(dir, what);
}

fn whitelist_settings() -> Settings {
    // A character list makes the ocrmypdf path write a Tesseract config file
    Settings {
        ocr_whitelist_chars: Some("0123456789".into()),
        ..Settings::default()
    }
}

#[tokio::test]
async fn success_paths_leave_the_temp_dir_empty() {
    let dir = tempfile::tempdir().unwrap();
    let ocr = service(dir.path());
    let pdf = fixture("test_files/single_page_v14.pdf");
    let started = std::time::Instant::now();

    ocr.extract_text_from_pdf_with_ocr(&pdf, &whitelist_settings(), started)
        .await
        .expect("ocrmypdf is required for this test");
    assert_empty(dir.path(), "ocrmypdf extraction");

    // Text found or not, the quick path's scratch file goes
    let _ = ocr.extract_pdf_text_quick(&pdf).await;
    assert_empty(dir.path(), "quick pdftotext extraction");

    // Preprocessed page images are not kept even when review saving is on
    let preprocessed = Settings {
        enable_image_preprocessing: true,
        save_processed_images: true,
        ..Settings::default()
    };
    ocr.extract_text_from_pdf_via_images(&pdf, &preprocessed, started, None)
        .await
        .expect("pdftoppm and tesseract are required for this test");
    assert_empty(dir.path(), "page-image extraction");

    let not_kept = Settings {
        enable_image_preprocessing: true,
        save_processed_images: false,
        ..Settings::default()
    };
    ocr.extract_text_from_image(&fixture("tests/test_images/test1.png"), &not_kept)
        .await
        .expect("tesseract is required for this test");
    assert_empty(dir.path(), "image extraction without review saving");
}

#[tokio::test]
async fn a_dropped_page_render_leaves_no_page_images() {
    let dir = tempfile::tempdir().unwrap();
    let ocr = service(dir.path());
    // 468 pages: pdftoppm is still rendering when the first page lands
    let pdf = fixture("test_files/porters-handbook_en.pdf");
    let params = TesseractParams::from_settings(&Settings::default());
    assert!(
        drop_once_files_appear(dir.path(), ocr.extract_pdf_pages_as_images(&pdf, &params)).await,
        "pdftoppm finished 468 pages before the first one was seen"
    );
    assert_empty_after_drop(dir.path(), "a dropped pdftoppm render").await;
}

#[tokio::test]
async fn a_dropped_ocrmypdf_run_leaves_no_files() {
    let dir = tempfile::tempdir().unwrap();
    let ocr = service(dir.path());
    let pdf = fixture("test_files/single_page_v14.pdf");
    let settings = whitelist_settings();
    let run = ocr.extract_text_from_pdf_with_ocr(&pdf, &settings, std::time::Instant::now());
    // Its sidecar text is written mid-run, before the output PDF is finished
    assert!(
        drop_once_a_file_matches(dir.path(), run, |name| name.starts_with("ocr_")).await,
        "ocrmypdf finished before its sidecar or output was seen"
    );
    assert_empty_after_drop(dir.path(), "a dropped ocrmypdf run").await;
}

#[tokio::test]
async fn a_dropped_quick_extraction_leaves_no_files() {
    let dir = tempfile::tempdir().unwrap();
    let ocr = service(dir.path());
    // Long enough that pdftotext is mid-write when the file first appears
    let pdf = fixture("test_files/porters-handbook_en.pdf");
    if drop_once_files_appear(dir.path(), ocr.extract_pdf_text_quick(&pdf)).await {
        assert_empty_after_drop(dir.path(), "a dropped pdftotext run").await;
    } else {
        // Finished first: the success path must still have cleaned up
        assert_empty(dir.path(), "quick pdftotext extraction");
    }
}
