//! Each OCR setting reaches the OCR engine: the pure mapping first, then the
//! engines themselves (tesseract and pdftoppm, with the `ocr` feature).

use super::*;

fn settings() -> Settings {
    Settings::default()
}

#[test]
fn engine_options_come_from_the_owners_settings() {
    let params = TesseractParams::from_settings(&Settings {
        ocr_page_segmentation_mode: 6,
        ocr_engine_mode: 1,
        ocr_dpi: 150,
        ocr_whitelist_chars: Some("0123456789".into()),
        ocr_blacklist_chars: Some("|".into()),
        ..settings()
    });
    assert_eq!(params.page_segmentation_mode, 6);
    assert_eq!(params.engine_mode, 1);
    assert_eq!(params.dpi, 150);
    assert_eq!(
        params.character_variables(),
        vec![
            ("tessedit_char_whitelist", "0123456789"),
            ("tessedit_char_blacklist", "|"),
        ]
    );
}

#[test]
fn out_of_range_values_fall_back_to_safe_ones() {
    let params = TesseractParams::from_settings(&Settings {
        ocr_page_segmentation_mode: 99,
        ocr_engine_mode: -1,
        ocr_dpi: 10_000,
        ocr_whitelist_chars: Some(String::new()),
        ..settings()
    });
    assert_eq!(params.page_segmentation_mode, 3);
    assert_eq!(params.engine_mode, 3);
    assert_eq!(params.dpi, MAX_OCR_DPI);
    assert!(params.character_variables().is_empty());
    assert_eq!(params.tesseract_config_file(), None);
    let low = TesseractParams::from_settings(&Settings {
        ocr_dpi: 1,
        ..settings()
    });
    assert_eq!(low.dpi, MIN_OCR_DPI);
}

#[test]
fn languages_combine_with_the_primary_first() {
    let s = Settings {
        preferred_languages: vec!["eng".into(), "deu".into(), "fra".into()],
        primary_language: "deu".into(),
        ..settings()
    };
    assert_eq!(language_combination(&s), "deu+eng+fra");
    let single = Settings {
        preferred_languages: vec!["spa".into()],
        ..settings()
    };
    assert_eq!(language_combination(&single), "spa");
    let legacy = Settings {
        preferred_languages: vec![],
        ocr_language: "ita".into(),
        ..settings()
    };
    assert_eq!(language_combination(&legacy), "ita");
}

#[test]
fn ocrmypdf_runs_with_the_owners_language_and_modes() {
    let params = TesseractParams::from_settings(&Settings {
        preferred_languages: vec!["deu".into()],
        ocr_engine_mode: 1,
        ocr_page_segmentation_mode: 4,
        ocr_whitelist_chars: Some("ABC".into()),
        ..settings()
    });
    let args = ocrmypdf_strategy_with_params(
        &["--force-ocr", "-O2", "--language", "eng"],
        &params,
        Some("/scratch/t.cfg"),
    );
    assert_eq!(
        args,
        [
            "--force-ocr",
            "-O2",
            "--language",
            "deu",
            "--tesseract-oem",
            "1",
            "--tesseract-pagesegmode",
            "4",
            "--tesseract-config",
            "/scratch/t.cfg",
        ]
    );
    assert!(
        !args.contains(&"eng".to_string()),
        "the strategy's default language is replaced"
    );
    assert_eq!(
        params.tesseract_config_file().as_deref(),
        Some("tessedit_char_whitelist ABC\n")
    );
}

#[test]
fn pdf_pages_render_at_the_owners_dpi() {
    let params = TesseractParams::from_settings(&Settings {
        ocr_dpi: 200,
        ..settings()
    });
    assert_eq!(params.pdftoppm_args(), ["-png", "-r", "200"]);
}

#[test]
fn preprocessing_respects_the_maximum_image_size_and_upscale_factor() {
    let limited = Settings {
        ocr_max_image_width: 800,
        ocr_max_image_height: 600,
        ..settings()
    };
    // Down to fit 800x600, aspect ratio kept.
    assert_eq!(preprocess_dimensions(1600, 1000, &limited), (800, 500));
    assert_eq!(preprocess_dimensions(1000, 1200, &limited), (500, 600));

    let upscaled = Settings {
        ocr_upscale_factor: 1.5,
        ..settings()
    };
    assert_eq!(preprocess_dimensions(1000, 800, &upscaled), (1500, 1200));
    // Upscaling still stops at the 2048-pixel ceiling.
    assert_eq!(preprocess_dimensions(1800, 900, &upscaled), (2048, 1024));

    // Defaults: a mid-sized image is left alone, a tiny one scaled up.
    assert_eq!(preprocess_dimensions(1000, 800, &settings()), (1000, 800));
    assert_eq!(preprocess_dimensions(200, 100, &settings()), (1200, 600));
}

#[test]
fn sharpening_follows_the_strength_setting() {
    // Strength 1 (and "automatic") is the classic 5-point sharpening kernel.
    let classic = [[0.0, -1.0, 0.0], [-1.0, 5.0, -1.0], [0.0, -1.0, 0.0]];
    assert_eq!(sharpening_kernel(1.0), classic);
    assert_eq!(sharpening_kernel(0.0), classic);
    assert_eq!(
        sharpening_kernel(0.5),
        [[0.0, -0.5, 0.0], [-0.5, 3.0, -0.5], [0.0, -0.5, 0.0]]
    );
    // Every kernel keeps flat areas unchanged (weights sum to 1).
    for strength in [0.25_f32, 1.5, 2.0] {
        let sum: f32 = sharpening_kernel(strength).iter().flatten().sum();
        assert!((sum - 1.0).abs() < 1e-6, "strength {strength}: {sum}");
    }
}

#[tokio::test]
async fn character_lists_are_written_to_a_tesseract_config_file() {
    let dir = tempfile::tempdir().unwrap();
    let dir_str = dir.path().to_str().unwrap();
    let none = TesseractParams::from_settings(&settings());
    assert_eq!(write_tesseract_config(&none, dir_str).await.unwrap(), None);

    let params = TesseractParams::from_settings(&Settings {
        ocr_blacklist_chars: Some("@#".into()),
        ..settings()
    });
    let path = write_tesseract_config(&params, dir_str)
        .await
        .unwrap()
        .expect("a config file");
    assert!(path.starts_with(dir_str), "{path} is under the temp dir");
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "tessedit_char_blacklist @#\n"
    );
}

#[cfg(feature = "ocr")]
mod engines {
    use super::*;
    use crate::ocr::enhanced::EnhancedOcrService;
    use std::path::Path;

    // `FileService::new` is deprecated for production wiring; a local
    // service is all these tests need.
    #[allow(deprecated)]
    fn service(temp_dir: &Path) -> EnhancedOcrService {
        let dir = temp_dir.to_string_lossy().into_owned();
        EnhancedOcrService::new(
            dir.clone(),
            crate::services::file_service::FileService::new(dir),
            100,
            100,
            300,
        )
    }

    fn fixture(path: &str) -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(path)
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn engine_modes_map_to_tesseracts() {
        use tesseract::OcrEngineMode;
        assert!(matches!(engine_mode(0), OcrEngineMode::TesseractOnly));
        assert!(matches!(engine_mode(1), OcrEngineMode::LstmOnly));
        assert!(matches!(
            engine_mode(2),
            OcrEngineMode::TesseractLstmCombined
        ));
        assert!(matches!(engine_mode(3), OcrEngineMode::Default));
    }

    /// The whitelist reaches Tesseract: the same image read with a digits-only
    /// whitelist yields no letters at all.
    #[tokio::test]
    async fn whitelist_restricts_what_tesseract_recognises() {
        let dir = tempfile::tempdir().unwrap();
        let ocr = service(dir.path());
        let image = fixture("tests/test_images/test1.png");

        let plain = ocr
            .extract_text_from_image(&image, &settings())
            .await
            .expect("tesseract reads the fixture");
        assert!(
            plain.text.chars().any(|c| c.is_alphabetic()),
            "without a whitelist the fixture's words are read: {:?}",
            plain.text
        );

        let digits_only = Settings {
            ocr_whitelist_chars: Some("0123456789".into()),
            ..settings()
        };
        let restricted = ocr
            .extract_text_from_image(&image, &digits_only)
            .await
            .expect("tesseract reads the fixture");
        assert!(
            !restricted.text.chars().any(|c| c.is_alphabetic()),
            "a digits-only whitelist yields no letters: {:?}",
            restricted.text
        );
    }

    /// PDF pages are rendered at the owner's DPI: doubling it doubles the
    /// rendered page's width.
    #[tokio::test]
    async fn pdf_page_images_follow_the_dpi_setting() {
        let dir = tempfile::tempdir().unwrap();
        let ocr = service(dir.path());
        if !ocr.is_pdftoppm_available().await {
            panic!("pdftoppm is required for this test (poppler-utils)");
        }
        let pdf = fixture("test_files/single_page_v14.pdf");
        let mut widths = Vec::new();
        for dpi in [72, 144] {
            let params = TesseractParams::from_settings(&Settings {
                ocr_dpi: dpi,
                ..settings()
            });
            let pages = ocr
                .extract_pdf_pages_as_images(&pdf, &params)
                .await
                .unwrap();
            assert!(
                pages
                    .iter()
                    .all(|p| p.starts_with(dir.path().to_str().unwrap())),
                "page images are written under the service's temp dir"
            );
            let (width, _) = image::image_dimensions(&pages[0]).unwrap();
            widths.push(width);
        }
        let ratio = widths[1] as f64 / widths[0] as f64;
        assert!((ratio - 2.0).abs() < 0.02, "widths {widths:?}");
    }

    /// ocrmypdf accepts every option the settings add (language, engine and
    /// segmentation modes, a character-list config file) and OCRs with them:
    /// a digits-only whitelist leaves no letters in the sidecar text.
    #[tokio::test]
    async fn ocrmypdf_runs_with_the_owners_options() {
        let dir = tempfile::tempdir().unwrap();
        let temp = dir.path().to_str().unwrap();
        let params = TesseractParams::from_settings(&Settings {
            ocr_engine_mode: 1,
            ocr_page_segmentation_mode: 3,
            ocr_whitelist_chars: Some("0123456789".into()),
            ..settings()
        });
        let config = write_tesseract_config(&params, temp).await.unwrap();
        let sidecar = format!("{temp}/out.txt");
        let args = crate::ocr::enhanced::ocrmypdf_ocr_command_args(
            &ocrmypdf_strategy_with_params(
                &crate::ocr::enhanced::ocrmypdf_strategy1_args(),
                &params,
                config.as_deref(),
            ),
            &sidecar,
            &fixture("test_files/single_page_v14.pdf"),
            &format!("{temp}/out.pdf"),
        );
        let output = tokio::process::Command::new("ocrmypdf")
            .args(&args)
            .output()
            .await
            .expect("ocrmypdf is required for this test");
        assert!(
            output.status.success(),
            "ocrmypdf {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = std::fs::read_to_string(&sidecar).unwrap();
        assert!(
            !text.chars().any(|c| c.is_alphabetic()),
            "a digits-only whitelist yields no letters: {text:?}"
        );
    }

    /// Preprocessing writes its processed image under the configured temp dir.
    #[tokio::test]
    async fn processed_images_go_to_the_configured_temp_dir() {
        let dir = tempfile::tempdir().unwrap();
        let ocr = service(dir.path());
        let keep = Settings {
            enable_image_preprocessing: true,
            save_processed_images: true,
            ..settings()
        };
        let result = ocr
            .extract_text_from_image(&fixture("tests/test_images/test1.png"), &keep)
            .await
            .unwrap();
        let processed = result.processed_image_path.expect("a processed image");
        assert!(
            processed.starts_with(dir.path().to_str().unwrap()),
            "{processed} is under {}",
            dir.path().display()
        );
        assert!(Path::new(&processed).exists());
    }
}
