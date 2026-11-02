#[cfg(feature = "ocr")]
mod unit_tests {
    use readur::services::file_service::FileService;
    use readur::storage::StorageConfig;
    use std::fmt::Write;
    use tempfile::TempDir;

    // A green first page and blue later pages distinguish the thumbnail from
    // both the red placeholder and a thumbnail of the wrong page.
    fn colored_pdf(page_count: usize) -> Vec<u8> {
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            format!(
                "<< /Type /Pages /Count {} /Kids [{}] >>",
                page_count,
                (0..page_count)
                    .map(|page| format!("{} 0 R", 3 + page * 2))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        ];
        for page in 0..page_count {
            objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 200 100] /Resources << >> /Contents {} 0 R >>",
            4 + page * 2
        ));
            let color = if page == 0 { "0 1 0" } else { "0 0 1" };
            let stream = format!("{color} rg 0 0 200 100 re f\n");
            objects.push(format!(
                "<< /Length {} >>\nstream\n{}endstream",
                stream.len(),
                stream
            ));
        }
        let mut pdf = "%PDF-1.4\n".to_string();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            writeln!(pdf, "{} 0 obj\n{}\nendobj", index + 1, object).unwrap();
        }
        let xref = pdf.len();
        writeln!(pdf, "xref\n0 {}\n0000000000 65535 f ", objects.len() + 1).unwrap();
        for offset in offsets {
            writeln!(pdf, "{offset:010} 00000 n ").unwrap();
        }
        write!(
            pdf,
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            objects.len() + 1,
            xref
        )
        .unwrap();
        pdf.into_bytes()
    }

    async fn assert_first_page_thumbnail(page_count: usize) {
        let directory = TempDir::new().unwrap();
        let upload_path = directory.path().to_string_lossy().into_owned();
        let service = FileService::from_config(
            StorageConfig::Local {
                upload_path: upload_path.clone(),
            },
            upload_path,
        )
        .await
        .unwrap();
        let pdf = service
            .save_file("pages.pdf", &colored_pdf(page_count))
            .await
            .unwrap();

        let bytes = service
            .get_or_generate_thumbnail(&pdf, "pages.pdf")
            .await
            .unwrap();

        let image = image::load_from_memory(&bytes).unwrap().to_rgb8();
        assert_eq!(image.dimensions(), (200, 100));
        let [red, green, blue] = image.get_pixel(100, 50).0;
        assert!(
            green > 240 && red < 15 && blue < 15,
            "expected a green first page, got {red}, {green}, {blue}"
        );
        tokio::fs::remove_file(&pdf).await.unwrap();
        let cached = service
            .get_or_generate_thumbnail(&pdf, "pages.pdf")
            .await
            .unwrap();
        assert_eq!(cached, bytes);
    }

    #[tokio::test]
    async fn pdf_thumbnail_one_page() {
        assert_first_page_thumbnail(1).await;
    }

    #[tokio::test]
    async fn pdf_thumbnail_ten_pages() {
        assert_first_page_thumbnail(10).await;
    }

    #[tokio::test]
    async fn pdf_thumbnail_one_hundred_pages() {
        assert_first_page_thumbnail(100).await;
    }

    #[tokio::test]
    async fn invalid_pdf_returns_placeholder() {
        let directory = TempDir::new().unwrap();
        let upload_path = directory.path().to_string_lossy().into_owned();
        let service = FileService::from_config(
            StorageConfig::Local {
                upload_path: upload_path.clone(),
            },
            upload_path,
        )
        .await
        .unwrap();
        let pdf = service
            .save_file("invalid.pdf", b"not a PDF")
            .await
            .unwrap();
        let bytes = service
            .get_or_generate_thumbnail(&pdf, "invalid.pdf")
            .await
            .unwrap();
        let image = image::load_from_memory(&bytes).unwrap().to_rgb8();
        assert_eq!(image.dimensions(), (200, 200));
        let [red, green, blue] = image.get_pixel(0, 0).0;
        assert!(
            red > green && red > blue,
            "expected a red placeholder, got {red}, {green}, {blue}"
        );
    }
}
