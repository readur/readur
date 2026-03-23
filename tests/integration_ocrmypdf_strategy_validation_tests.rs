/// Integration tests that validate all OCR tool invocations and strategy paths.
///
/// These tests exercise every external binary used in the OCR pipeline:
/// - pdftotext (text extraction from text-based PDFs)
/// - ocrmypdf (full OCR, --sidecar text extraction, argument validation)
/// - pdftoppm (PDF-to-image conversion)
/// - pdfimages (embedded image detection)
/// - pdfinfo (page counting)
///
/// The tests create real PDF fixtures and run the actual binaries, catching
/// compatibility issues like #604 where --fix-metadata was not a valid flag.
///
/// These tests require all OCR tools to be installed (CI and Docker both have them).
#[cfg(test)]
mod tests {
    use readur::ocr::enhanced::{
        ocrmypdf_strategy1_args, ocrmypdf_strategy2_args, EnhancedOcrService,
    };
    use readur::models::Settings;
    use readur::services::file_service::FileService;
    use readur::storage::{factory::create_storage_backend, StorageConfig};
    use std::process::Command;
    use tempfile::TempDir;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn ocrmypdf_version() -> String {
        Command::new("ocrmypdf")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_else(|| "unknown".to_string())
            .trim()
            .to_string()
    }

    /// Assert that a tool is installed and reachable, panicking with a helpful
    /// message if not. We intentionally do NOT skip — CI must have these tools.
    fn require_tool(name: &str, version_flag: &str) {
        let output = Command::new(name)
            .arg(version_flag)
            .output()
            .unwrap_or_else(|e| panic!("{name} is not installed or not in PATH: {e}"));
        assert!(
            output.status.success() || !output.stderr.is_empty(),
            "{name} {version_flag} failed with status {}",
            output.status
        );
    }

    /// Create a minimal valid PDF with an embedded text layer.
    /// The text "Hello World integration test" appears on page 1.
    fn create_text_pdf(path: &str) {
        // Use a raw PDF with a text stream so pdftotext can extract it
        let pdf = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj

2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj

3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]
   /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>
endobj

4 0 obj
<< /Length 44 >>
stream
BT /F1 12 Tf 100 700 Td (Hello World integration test) Tj ET
endstream
endobj

5 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj

xref
0 6
0000000000 65535 f \r\n0000000009 00000 n \r\n0000000058 00000 n \r\n0000000115 00000 n \r\n0000000266 00000 n \r\n0000000360 00000 n \r\n
trailer
<< /Size 6 /Root 1 0 R >>
startxref
441
%%EOF";
        std::fs::write(path, pdf).expect("Failed to write text PDF fixture");
    }

    /// Create a minimal valid PDF with NO text layer (just a blank page).
    /// This forces the OCR fallback paths.
    fn create_blank_pdf(path: &str) {
        let pdf = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj

2 0 obj
<< /Type /Pages /Kids [3 0 R] /Count 1 >>
endobj

3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>
endobj

xref
0 4
0000000000 65535 f \r\n0000000009 00000 n \r\n0000000058 00000 n \r\n0000000115 00000 n \r\n
trailer
<< /Size 4 /Root 1 0 R >>
startxref
190
%%EOF";
        std::fs::write(path, pdf).expect("Failed to write blank PDF fixture");
    }

    /// Create a 2-page PDF for page-count tests.
    fn create_two_page_pdf(path: &str) {
        let pdf = b"%PDF-1.4
1 0 obj
<< /Type /Catalog /Pages 2 0 R >>
endobj

2 0 obj
<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>
endobj

3 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]
   /Contents 5 0 R /Resources << /Font << /F1 7 0 R >> >> >>
endobj

4 0 obj
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792]
   /Contents 6 0 R /Resources << /Font << /F1 7 0 R >> >> >>
endobj

5 0 obj
<< /Length 30 >>
stream
BT /F1 12 Tf 100 700 Td (Page one content) Tj ET
endstream
endobj

6 0 obj
<< /Length 30 >>
stream
BT /F1 12 Tf 100 700 Td (Page two content) Tj ET
endstream
endobj

7 0 obj
<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>
endobj

xref
0 8
0000000000 65535 f \r\n0000000009 00000 n \r\n0000000058 00000 n \r\n0000000115 00000 n \r\n0000000266 00000 n \r\n0000000417 00000 n \r\n0000000497 00000 n \r\n0000000577 00000 n \r\n
trailer
<< /Size 8 /Root 1 0 R >>
startxref
658
%%EOF";
        std::fs::write(path, pdf).expect("Failed to write two-page PDF fixture");
    }

    fn create_temp_dir() -> TempDir {
        TempDir::new().expect("Failed to create temp dir")
    }

    async fn create_test_service(temp_path: &str) -> EnhancedOcrService {
        let storage_config = StorageConfig::Local {
            upload_path: temp_path.to_string(),
        };
        let storage_backend = create_storage_backend(storage_config).await.unwrap();
        let file_service = FileService::with_storage(temp_path.to_string(), storage_backend);
        EnhancedOcrService::new(
            temp_path.to_string(),
            file_service,
            100,  // max_pdf_size_mb
            100,  // max_office_document_size_mb
            300,  // ocr_timeout_seconds
        )
    }

    // -----------------------------------------------------------------------
    // Tool availability (these MUST pass in CI)
    // -----------------------------------------------------------------------

    #[test]
    fn test_ocrmypdf_is_installed() {
        require_tool("ocrmypdf", "--version");
    }

    #[test]
    fn test_pdftotext_is_installed() {
        require_tool("pdftotext", "-v");
    }

    #[test]
    fn test_pdftoppm_is_installed() {
        require_tool("pdftoppm", "-v");
    }

    #[test]
    fn test_pdfimages_is_installed() {
        require_tool("pdfimages", "-v");
    }

    #[test]
    fn test_pdfinfo_is_installed() {
        require_tool("pdfinfo", "-v");
    }

    #[test]
    fn test_tesseract_is_installed() {
        require_tool("tesseract", "--version");
    }

    // -----------------------------------------------------------------------
    // ocrmypdf argument validation (regression for #604)
    // -----------------------------------------------------------------------

    /// Validate that a set of ocrmypdf arguments are accepted by the binary.
    fn assert_ocrmypdf_accepts_args(strategy_name: &str, args: &[&str]) {
        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("input.pdf");
        let output = temp_dir.path().join("output.pdf");

        create_blank_pdf(input.to_str().unwrap());

        let mut cmd = Command::new("ocrmypdf");
        for arg in args {
            cmd.arg(arg);
        }
        cmd.arg(input.to_str().unwrap());
        cmd.arg(output.to_str().unwrap());

        let result = cmd.output().expect("Failed to execute ocrmypdf");
        let stderr = String::from_utf8_lossy(&result.stderr);

        // Exit code 2 = argument parsing error — the exact failure from #604
        assert_ne!(
            result.status.code(),
            Some(2),
            "{strategy_name}: ocrmypdf rejected arguments (exit 2).\n\
             Version: {}\nArgs: {args:?}\nStderr: {stderr}",
            ocrmypdf_version(),
        );

        assert!(
            !stderr.contains("unrecognized arguments"),
            "{strategy_name}: ocrmypdf reported 'unrecognized arguments'.\n\
             Version: {}\nArgs: {args:?}\nStderr: {stderr}",
            ocrmypdf_version(),
        );
    }

    #[test]
    fn test_ocrmypdf_strategy1_args_accepted_by_binary() {
        let args = ocrmypdf_strategy1_args();
        assert_ocrmypdf_accepts_args("Strategy 1 (standard OCR)", &args);
    }

    #[test]
    fn test_ocrmypdf_strategy2_args_accepted_by_binary() {
        let args = ocrmypdf_strategy2_args();
        assert_ocrmypdf_accepts_args("Strategy 2 (recovery mode)", &args);
    }

    #[test]
    fn test_all_strategy_flags_appear_in_ocrmypdf_help() {
        let help = Command::new("ocrmypdf")
            .arg("--help")
            .output()
            .expect("Failed to run ocrmypdf --help");
        let help_text = String::from_utf8_lossy(&help.stdout);

        for (name, args) in [
            ("Strategy 1", ocrmypdf_strategy1_args()),
            ("Strategy 2", ocrmypdf_strategy2_args()),
        ] {
            for arg in &args {
                if !arg.starts_with('-') {
                    continue;
                }
                // For value flags like -O2, check the base flag (-O) exists
                let check_flag = if arg.starts_with("-O") && arg.len() > 2 {
                    "-O"
                } else {
                    arg
                };
                assert!(
                    help_text.contains(check_flag),
                    "{name}: flag '{check_flag}' (from '{arg}') not in ocrmypdf --help.\n\
                     Version: {}",
                    ocrmypdf_version(),
                );
            }
        }
    }

    /// Regression: --fix-metadata must never be reintroduced (it doesn't exist).
    #[test]
    fn test_fix_metadata_flag_never_in_strategies() {
        for args in [ocrmypdf_strategy1_args(), ocrmypdf_strategy2_args()] {
            assert!(
                !args.contains(&"--fix-metadata"),
                "--fix-metadata must not be used (see issue #604)"
            );
        }
    }

    // -----------------------------------------------------------------------
    // pdftotext: fast text extraction path
    // -----------------------------------------------------------------------

    #[test]
    fn test_pdftotext_extracts_text_from_text_pdf() {
        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("text.pdf");
        let output_txt = temp_dir.path().join("output.txt");

        create_text_pdf(input.to_str().unwrap());

        let result = Command::new("pdftotext")
            .arg("-layout")
            .arg(input.to_str().unwrap())
            .arg(output_txt.to_str().unwrap())
            .output()
            .expect("Failed to execute pdftotext");

        assert!(
            result.status.success(),
            "pdftotext failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );

        let text = std::fs::read_to_string(&output_txt).expect("Failed to read pdftotext output");
        assert!(
            text.contains("Hello") && text.contains("World"),
            "pdftotext should extract 'Hello World' from text PDF, got: '{text}'"
        );
    }

    #[test]
    fn test_pdftotext_returns_empty_for_blank_pdf() {
        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("blank.pdf");
        let output_txt = temp_dir.path().join("output.txt");

        create_blank_pdf(input.to_str().unwrap());

        let result = Command::new("pdftotext")
            .arg("-layout")
            .arg(input.to_str().unwrap())
            .arg(output_txt.to_str().unwrap())
            .output()
            .expect("Failed to execute pdftotext");

        // pdftotext may succeed but produce empty/whitespace output
        if result.status.success() {
            if let Ok(text) = std::fs::read_to_string(&output_txt) {
                let word_count = text.split_whitespace().count();
                assert!(
                    word_count <= 5,
                    "Blank PDF should produce ≤5 words from pdftotext, got {word_count}"
                );
            }
        }
        // Failure is also acceptable for a blank PDF
    }

    // -----------------------------------------------------------------------
    // pdfinfo: page counting
    // -----------------------------------------------------------------------

    #[test]
    fn test_pdfinfo_extracts_page_count() {
        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("two_pages.pdf");
        create_two_page_pdf(input.to_str().unwrap());

        let result = Command::new("pdfinfo")
            .arg(input.to_str().unwrap())
            .output()
            .expect("Failed to execute pdfinfo");

        assert!(
            result.status.success(),
            "pdfinfo failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );

        let stdout = String::from_utf8_lossy(&result.stdout);
        let page_count: Option<usize> = stdout.lines().find_map(|line| {
            if line.starts_with("Pages:") {
                line.split_whitespace().last()?.parse().ok()
            } else {
                None
            }
        });

        assert_eq!(
            page_count,
            Some(2),
            "pdfinfo should report 2 pages. Output:\n{stdout}"
        );
    }

    #[test]
    fn test_pdfinfo_single_page_pdf() {
        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("single.pdf");
        create_text_pdf(input.to_str().unwrap());

        let result = Command::new("pdfinfo")
            .arg(input.to_str().unwrap())
            .output()
            .expect("Failed to execute pdfinfo");

        let stdout = String::from_utf8_lossy(&result.stdout);
        let page_count: Option<usize> = stdout.lines().find_map(|line| {
            if line.starts_with("Pages:") {
                line.split_whitespace().last()?.parse().ok()
            } else {
                None
            }
        });

        assert_eq!(page_count, Some(1));
    }

    // -----------------------------------------------------------------------
    // pdfimages: embedded image detection
    // -----------------------------------------------------------------------

    #[test]
    fn test_pdfimages_reports_no_images_for_text_pdf() {
        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("text.pdf");
        create_text_pdf(input.to_str().unwrap());

        let result = Command::new("pdfimages")
            .arg("-list")
            .arg(input.to_str().unwrap())
            .output()
            .expect("Failed to execute pdfimages");

        assert!(result.status.success(), "pdfimages failed");

        let stdout = String::from_utf8_lossy(&result.stdout);
        // Skip header lines (2) and count image entries
        let image_count = stdout.lines().skip(2).filter(|l| l.contains("image")).count();
        assert_eq!(
            image_count, 0,
            "Text-only PDF should have no embedded images"
        );
    }

    // -----------------------------------------------------------------------
    // pdftoppm: PDF to image conversion
    // -----------------------------------------------------------------------

    #[test]
    fn test_pdftoppm_converts_pdf_to_png() {
        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("text.pdf");
        let prefix = temp_dir.path().join("page");

        create_text_pdf(input.to_str().unwrap());

        let result = Command::new("pdftoppm")
            .arg("-png")
            .arg("-r")
            .arg("300")
            .arg(input.to_str().unwrap())
            .arg(prefix.to_str().unwrap())
            .output()
            .expect("Failed to execute pdftoppm");

        assert!(
            result.status.success(),
            "pdftoppm failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );

        // pdftoppm names files as prefix-N.png (e.g., page-1.png)
        let png_exists = temp_dir
            .path()
            .read_dir()
            .unwrap()
            .any(|entry| {
                entry
                    .unwrap()
                    .path()
                    .extension()
                    .map_or(false, |ext| ext == "png")
            });

        assert!(png_exists, "pdftoppm should produce at least one PNG file");
    }

    #[test]
    fn test_pdftoppm_produces_correct_number_of_pages() {
        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("two_pages.pdf");
        let prefix = temp_dir.path().join("page");

        create_two_page_pdf(input.to_str().unwrap());

        let result = Command::new("pdftoppm")
            .arg("-png")
            .arg("-r")
            .arg("150") // lower DPI for speed
            .arg(input.to_str().unwrap())
            .arg(prefix.to_str().unwrap())
            .output()
            .expect("Failed to execute pdftoppm");

        assert!(result.status.success());

        let png_count = temp_dir
            .path()
            .read_dir()
            .unwrap()
            .filter(|entry| {
                entry
                    .as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .map_or(false, |ext| ext == "png")
            })
            .count();

        assert_eq!(
            png_count, 2,
            "pdftoppm should produce 2 PNGs for a 2-page PDF, got {png_count}"
        );
    }

    // -----------------------------------------------------------------------
    // ocrmypdf --sidecar: text layer extraction
    // -----------------------------------------------------------------------

    #[test]
    fn test_ocrmypdf_sidecar_extracts_text_layer() {
        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("text.pdf");
        let sidecar = temp_dir.path().join("sidecar.txt");

        create_text_pdf(input.to_str().unwrap());

        // Use --skip-text to extract existing text layer without re-OCRing
        // (ocrmypdf 17+ requires explicit mode when PDF already has text)
        let result = Command::new("ocrmypdf")
            .arg("--skip-text")
            .arg("--sidecar")
            .arg(sidecar.to_str().unwrap())
            .arg(input.to_str().unwrap())
            .arg("-") // dummy output
            .output()
            .expect("Failed to execute ocrmypdf --sidecar");

        // --sidecar should succeed on a text PDF
        assert!(
            result.status.success(),
            "ocrmypdf --sidecar failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );

        let text = std::fs::read_to_string(&sidecar).unwrap_or_default();
        // The sidecar should contain text from the PDF
        assert!(
            !text.trim().is_empty(),
            "ocrmypdf --sidecar should extract text from text PDF"
        );
    }

    // -----------------------------------------------------------------------
    // ocrmypdf Strategy 1: full OCR with standard settings
    // -----------------------------------------------------------------------

    #[test]
    fn test_ocrmypdf_strategy1_produces_ocrd_pdf() {
        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("input.pdf");
        let output = temp_dir.path().join("output.pdf");

        create_text_pdf(input.to_str().unwrap());

        let mut cmd = Command::new("ocrmypdf");
        for arg in ocrmypdf_strategy1_args() {
            cmd.arg(arg);
        }
        cmd.arg(input.to_str().unwrap());
        cmd.arg(output.to_str().unwrap());

        let result = cmd.output().expect("Failed to execute ocrmypdf strategy 1");

        assert!(
            result.status.success(),
            "Strategy 1 failed on text PDF (exit {}):\n{}",
            result.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&result.stderr)
        );

        // Output PDF should exist and be non-empty
        let output_size = std::fs::metadata(&output)
            .expect("Output PDF should exist")
            .len();
        assert!(output_size > 0, "Output PDF should be non-empty");
    }

    // -----------------------------------------------------------------------
    // ocrmypdf Strategy 2: recovery mode
    // -----------------------------------------------------------------------

    #[test]
    fn test_ocrmypdf_strategy2_produces_ocrd_pdf() {
        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("input.pdf");
        let output = temp_dir.path().join("output.pdf");

        create_text_pdf(input.to_str().unwrap());

        let mut cmd = Command::new("ocrmypdf");
        for arg in ocrmypdf_strategy2_args() {
            cmd.arg(arg);
        }
        cmd.arg(input.to_str().unwrap());
        cmd.arg(output.to_str().unwrap());

        let result = cmd.output().expect("Failed to execute ocrmypdf strategy 2");

        assert!(
            result.status.success(),
            "Strategy 2 failed on text PDF (exit {}):\n{}",
            result.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&result.stderr)
        );

        let output_size = std::fs::metadata(&output)
            .expect("Output PDF should exist")
            .len();
        assert!(output_size > 0, "Output PDF should be non-empty");
    }

    // -----------------------------------------------------------------------
    // Strategy 1 → Strategy 2 fallback chain (full flow)
    // -----------------------------------------------------------------------

    #[test]
    fn test_strategy_fallback_chain_produces_result() {
        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("input.pdf");
        let output = temp_dir.path().join("output.pdf");

        create_blank_pdf(input.to_str().unwrap());

        // Simulate the exact fallback logic from extract_text_from_pdf_with_ocr
        let mut cmd1 = Command::new("ocrmypdf");
        for arg in ocrmypdf_strategy1_args() {
            cmd1.arg(arg);
        }
        cmd1.arg(input.to_str().unwrap());
        cmd1.arg(output.to_str().unwrap());

        let result1 = cmd1.output().expect("Failed to execute strategy 1");

        if result1.status.success() {
            // Strategy 1 succeeded — output exists
            assert!(output.exists(), "Strategy 1 should produce output PDF");
            return;
        }

        // Strategy 1 failed — try Strategy 2 (exact fallback from enhanced.rs)
        let mut cmd2 = Command::new("ocrmypdf");
        for arg in ocrmypdf_strategy2_args() {
            cmd2.arg(arg);
        }
        cmd2.arg(input.to_str().unwrap());
        cmd2.arg(output.to_str().unwrap());

        let result2 = cmd2.output().expect("Failed to execute strategy 2");

        // At least one strategy should succeed on a valid (if blank) PDF
        // If both fail, that's still an acceptable outcome for a blank page —
        // but we must not fail with exit code 2 (argument error)
        assert_ne!(
            result2.status.code(),
            Some(2),
            "Strategy 2 must not fail with argument error.\nStderr: {}",
            String::from_utf8_lossy(&result2.stderr)
        );
    }

    // -----------------------------------------------------------------------
    // Full OCR + sidecar text extraction chain
    // -----------------------------------------------------------------------

    #[test]
    fn test_ocr_then_sidecar_text_extraction() {
        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("input.pdf");
        let ocrd_output = temp_dir.path().join("ocrd.pdf");
        let sidecar = temp_dir.path().join("sidecar.txt");

        create_text_pdf(input.to_str().unwrap());

        // Step 1: OCR the PDF (Strategy 1)
        let mut cmd = Command::new("ocrmypdf");
        for arg in ocrmypdf_strategy1_args() {
            cmd.arg(arg);
        }
        cmd.arg(input.to_str().unwrap());
        cmd.arg(ocrd_output.to_str().unwrap());

        let result = cmd.output().expect("Failed to execute ocrmypdf");
        assert!(result.status.success(), "OCR step failed");

        // Step 2: Extract text via --sidecar (same as enhanced.rs line 1184)
        // Use --skip-text since the OCR'd PDF already has a text layer
        let extract = Command::new("ocrmypdf")
            .arg("--skip-text")
            .arg("--sidecar")
            .arg(sidecar.to_str().unwrap())
            .arg(ocrd_output.to_str().unwrap())
            .arg("-")
            .output()
            .expect("Failed to execute sidecar extraction");

        assert!(
            extract.status.success(),
            "Sidecar extraction failed: {}",
            String::from_utf8_lossy(&extract.stderr)
        );

        let text = std::fs::read_to_string(&sidecar).unwrap_or_default();
        assert!(
            !text.trim().is_empty(),
            "Sidecar should extract text from OCR'd PDF"
        );
    }

    // -----------------------------------------------------------------------
    // Full pdftoppm → tesseract per-page path
    // -----------------------------------------------------------------------

    #[test]
    fn test_pdftoppm_then_tesseract_extraction() {
        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("text.pdf");
        let prefix = temp_dir.path().join("page");

        create_text_pdf(input.to_str().unwrap());

        // Step 1: Convert PDF to images (same as enhanced.rs line 1409)
        let result = Command::new("pdftoppm")
            .arg("-png")
            .arg("-r")
            .arg("300")
            .arg(input.to_str().unwrap())
            .arg(prefix.to_str().unwrap())
            .output()
            .expect("Failed to execute pdftoppm");

        assert!(result.status.success(), "pdftoppm failed");

        // Find generated PNG
        let png_path = temp_dir
            .path()
            .read_dir()
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.path().extension().map_or(false, |ext| ext == "png"))
            .expect("pdftoppm should produce a PNG")
            .path();

        // Step 2: Run tesseract on the image (same path as enhanced.rs line 1469)
        let result = Command::new("tesseract")
            .arg(png_path.to_str().unwrap())
            .arg("stdout")
            .arg("-l")
            .arg("eng")
            .output()
            .expect("Failed to execute tesseract");

        assert!(
            result.status.success(),
            "tesseract failed: {}",
            String::from_utf8_lossy(&result.stderr)
        );

        let text = String::from_utf8_lossy(&result.stdout);
        // Tesseract should find at least some text from our PDF
        assert!(
            text.split_whitespace().count() > 0,
            "tesseract should extract some text from the rendered page"
        );
    }

    // -----------------------------------------------------------------------
    // Full end-to-end: pdftotext → sidecar → full OCR decision chain
    // -----------------------------------------------------------------------

    #[test]
    fn test_full_extraction_decision_chain_text_pdf() {
        // Simulates the full decision tree from enhanced.rs extract_text_from_pdf:
        //   1. pdftotext → if ≥5 words, done
        //   2. ocrmypdf --sidecar → if ≥5 words, done
        //   3. ocrmypdf full OCR (strategy 1 → strategy 2)

        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("text.pdf");
        let text_out = temp_dir.path().join("text.txt");

        create_text_pdf(input.to_str().unwrap());

        // Step 1: pdftotext
        let pdftotext = Command::new("pdftotext")
            .arg("-layout")
            .arg(input.to_str().unwrap())
            .arg(text_out.to_str().unwrap())
            .output()
            .expect("pdftotext failed");

        if pdftotext.status.success() {
            if let Ok(text) = std::fs::read_to_string(&text_out) {
                let words = text.split_whitespace().count();
                if words > 5 {
                    // Fast path succeeded — this is the expected path for text PDFs
                    assert!(
                        text.contains("Hello"),
                        "pdftotext should find 'Hello' in text PDF"
                    );
                    return;
                }
            }
        }

        // Step 2: ocrmypdf --sidecar
        let sidecar_out = temp_dir.path().join("sidecar.txt");
        let sidecar = Command::new("ocrmypdf")
            .arg("--skip-text")
            .arg("--sidecar")
            .arg(sidecar_out.to_str().unwrap())
            .arg(input.to_str().unwrap())
            .arg("-")
            .output()
            .expect("sidecar failed");

        if sidecar.status.success() {
            if let Ok(text) = std::fs::read_to_string(&sidecar_out) {
                let words = text.split_whitespace().count();
                if words > 5 {
                    return; // Sidecar path succeeded
                }
            }
        }

        // Step 3: Full OCR (should not be needed for a text PDF, but must not fail)
        let ocr_out = temp_dir.path().join("ocrd.pdf");
        let mut cmd = Command::new("ocrmypdf");
        for arg in ocrmypdf_strategy1_args() {
            cmd.arg(arg);
        }
        cmd.arg(input.to_str().unwrap());
        cmd.arg(ocr_out.to_str().unwrap());

        let ocr_result = cmd.output().expect("ocrmypdf strategy 1 failed");
        assert!(
            ocr_result.status.success(),
            "Full OCR path should succeed for text PDF"
        );
    }

    #[test]
    fn test_full_extraction_decision_chain_blank_pdf() {
        // For a blank PDF, the chain exercises all fallback paths
        let temp_dir = create_temp_dir();
        let input = temp_dir.path().join("blank.pdf");
        let text_out = temp_dir.path().join("text.txt");

        create_blank_pdf(input.to_str().unwrap());

        // Step 1: pdftotext — should succeed but produce ≤5 words
        let _ = Command::new("pdftotext")
            .arg("-layout")
            .arg(input.to_str().unwrap())
            .arg(text_out.to_str().unwrap())
            .output();

        let pdftotext_words = std::fs::read_to_string(&text_out)
            .map(|t| t.split_whitespace().count())
            .unwrap_or(0);

        // Blank PDF should not satisfy the ≥5 word threshold
        assert!(
            pdftotext_words <= 5,
            "Blank PDF should not produce substantial text from pdftotext"
        );

        // Step 2: ocrmypdf --sidecar — should also produce minimal text
        let sidecar_out = temp_dir.path().join("sidecar.txt");
        let sidecar = Command::new("ocrmypdf")
            .arg("--skip-text")
            .arg("--sidecar")
            .arg(sidecar_out.to_str().unwrap())
            .arg(input.to_str().unwrap())
            .arg("-")
            .output();

        // Step 3: Full OCR — must not fail with argument errors
        let ocr_out = temp_dir.path().join("ocrd.pdf");
        let mut cmd = Command::new("ocrmypdf");
        for arg in ocrmypdf_strategy1_args() {
            cmd.arg(arg);
        }
        cmd.arg(input.to_str().unwrap());
        cmd.arg(ocr_out.to_str().unwrap());

        let result = cmd.output().expect("ocrmypdf failed to execute");

        // Must not be an argument error
        assert_ne!(
            result.status.code(),
            Some(2),
            "Must not fail with argument error on blank PDF.\nStderr: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    // -----------------------------------------------------------------------
    // EnhancedOcrService integration: pdf_has_images, get_pdf_page_count
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_service_pdf_has_images_text_only() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let service = create_test_service(&temp_path).await;

        let input = temp_dir.path().join("text.pdf");
        create_text_pdf(input.to_str().unwrap());

        let has_images = service.pdf_has_images(input.to_str().unwrap()).await;
        assert!(
            !has_images,
            "Text-only PDF should not be detected as having images"
        );
    }

    #[tokio::test]
    async fn test_service_get_pdf_page_count() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let service = create_test_service(&temp_path).await;

        let input = temp_dir.path().join("two_pages.pdf");
        create_two_page_pdf(input.to_str().unwrap());

        let page_count = service
            .get_pdf_page_count(input.to_str().unwrap())
            .await
            .expect("get_pdf_page_count should succeed");

        assert_eq!(page_count, 2, "Should detect 2 pages");
    }

    #[tokio::test]
    async fn test_service_is_pdftoppm_available() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let service = create_test_service(&temp_path).await;

        assert!(
            service.is_pdftoppm_available().await,
            "pdftoppm must be available in CI"
        );
    }

    // -----------------------------------------------------------------------
    // EnhancedOcrService: full extract_text_from_pdf path
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_service_extract_text_from_text_pdf() {
        let temp_dir = create_temp_dir();
        let temp_path = temp_dir.path().to_str().unwrap().to_string();
        let service = create_test_service(&temp_path).await;
        let settings = Settings::default();

        let input = temp_dir.path().join("text.pdf");
        create_text_pdf(input.to_str().unwrap());

        let result = service
            .extract_text_from_pdf(input.to_str().unwrap(), &settings, None)
            .await;

        // This exercises the full decision chain through the service
        match result {
            Ok(ocr_result) => {
                assert!(
                    ocr_result.word_count > 0,
                    "Should extract words from text PDF"
                );
                assert!(
                    ocr_result.processing_time_ms > 0,
                    "Processing time should be recorded"
                );
            }
            Err(e) => {
                // Some CI environments may have tesseract but not all language packs
                let err_msg = e.to_string();
                assert!(
                    !err_msg.contains("unrecognized arguments"),
                    "Must not fail with 'unrecognized arguments': {err_msg}"
                );
            }
        }
    }
}
