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
    drop_once_dir_shows(dir, fut, |dir| entries(dir).iter().any(|name| wanted(name))).await
}

/// Polls `fut` until `shows` holds for `dir`, then drops it. Returns false if
/// the future finished first.
async fn drop_once_dir_shows<F: Future>(dir: &Path, fut: F, shows: impl Fn(&Path) -> bool) -> bool {
    let appeared = async {
        while !shows(dir) {
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
    let result = ocr
        .extract_text_from_image(&fixture("tests/test_images/test1.png"), &not_kept)
        .await
        .expect("tesseract is required for this test");
    assert_empty(dir.path(), "image extraction without review saving");
    assert_eq!(
        result.processed_image_path, None,
        "a preprocessed image that was not kept is not handed out"
    );
}

#[tokio::test]
async fn a_dropped_page_render_leaves_no_page_images() {
    let dir = tempfile::tempdir().unwrap();
    let ocr = service(dir.path());
    // 468 pages: pdftoppm is still rendering when the first page lands
    let pdf = fixture("test_files/porters-handbook_en.pdf");
    let params = TesseractParams::from_settings(&Settings::default());
    // Pages land in the render's own directory under the temp dir
    let a_page_landed = |dir: &Path| {
        std::fs::read_dir(dir).unwrap().any(|entry| {
            let path = entry.unwrap().path();
            path.is_dir() && std::fs::read_dir(&path).is_ok_and(|mut pages| pages.next().is_some())
        })
    };
    let render = ocr.extract_pdf_pages_as_images(&pdf, &params);
    let dropped_mid_render = drop_once_dir_shows(dir.path(), render, a_page_landed).await;
    assert!(
        dropped_mid_render,
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

/// Two documents' quick extractions running at once each read their own
/// sidecar: a name shared between concurrent calls (pid + millisecond) hands
/// one job the other's text, or lets one guard delete the other's file.
#[tokio::test]
async fn concurrent_quick_extractions_read_their_own_text() {
    let dir = tempfile::tempdir().unwrap();
    let ocr = service(dir.path());
    let pdfs = [
        fixture("test_files/complex_content.pdf"),
        fixture("test_files/multipage_test.pdf"),
    ];
    let mut expected = Vec::new();
    for pdf in &pdfs {
        expected.push(ocr.extract_pdf_text_quick(pdf).await.expect("pdftotext").0);
    }
    assert_ne!(expected[0], expected[1], "the fixtures must differ");
    let runs = (0..16).map(|i| {
        let ocr = &ocr;
        let pdf = &pdfs[i % 2];
        async move { (i % 2, ocr.extract_pdf_text_quick(pdf).await) }
    });
    for (which, got) in futures::future::join_all(runs).await {
        let text = got.expect("each run finds its own text").0;
        assert_eq!(
            text, expected[which],
            "run of fixture {which} read another document"
        );
    }
    assert_empty(dir.path(), "concurrent quick extractions");
}

/// The same for [`crate::ocr::OcrService::extract_text_from_pdf`], whose
/// sidecar used to be one fixed name per process.
#[tokio::test]
async fn concurrent_service_pdf_extractions_read_their_own_text() {
    let dir = tempfile::tempdir().unwrap();
    let ocr = crate::ocr::OcrService::new_with_config(crate::ocr::OcrConfig {
        temp_dir: dir.path().to_string_lossy().into_owned(),
    });
    let pdfs = [
        fixture("test_files/complex_content.pdf"),
        fixture("test_files/multipage_test.pdf"),
    ];
    let mut expected = Vec::new();
    for pdf in &pdfs {
        expected.push(ocr.extract_text_from_pdf(pdf).await.expect("pdftotext"));
    }
    assert_ne!(expected[0], expected[1], "the fixtures must differ");
    let runs = (0..16).map(|i| {
        let ocr = &ocr;
        let pdf = &pdfs[i % 2];
        async move { (i % 2, ocr.extract_text_from_pdf(pdf).await) }
    });
    for (which, got) in futures::future::join_all(runs).await {
        let text = got.expect("each run finds its own text");
        assert_eq!(
            text, expected[which],
            "run of fixture {which} read another document"
        );
    }
    assert_empty(dir.path(), "concurrent service extractions");
}
