//! Unit tests for the bundled test images in `test_utils`.

use super::*;

#[test]
fn test_image_paths_are_valid() {
    let images = get_test_images();
    assert_eq!(images.len(), 9);

    for (i, image) in images.iter().enumerate() {
        assert_eq!(
            image.filename,
            format!(
                "test{}.{}",
                i + 1,
                if image.mime_type == "image/png" {
                    "png"
                } else if image.filename.ends_with(".jpg") {
                    "jpg"
                } else {
                    "jpeg"
                }
            )
        );
        assert!(image
            .expected_content
            .starts_with(&format!("Test {}", i + 1)));
    }
}

#[test]
fn test_get_specific_image() {
    let image1 = get_test_image(1).unwrap();
    assert_eq!(image1.filename, "test1.png");
    assert_eq!(image1.mime_type, "image/png");
    assert!(image1.expected_content.contains("Test 1"));

    let image5 = get_test_image(5).unwrap();
    assert_eq!(image5.filename, "test5.jpg");
    assert_eq!(image5.mime_type, "image/jpeg");
    assert!(image5.expected_content.contains("Test 5"));

    // Invalid numbers should return None
    assert!(get_test_image(0).is_none());
    assert!(get_test_image(10).is_none());
}
