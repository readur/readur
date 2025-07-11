// This is a helper script to create test images
use image::{ImageBuffer, Rgb, DynamicImage};
use std::path::Path;

pub fn create_test_images() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple 100x200 RGB image (portrait)
    let mut img = ImageBuffer::new(100, 200);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let r = (x * 255 / 100) as u8;
        let g = (y * 255 / 200) as u8;
        let b = 128;
        *pixel = Rgb([r, g, b]);
    }
    
    let dynamic_img = DynamicImage::ImageRgb8(img);
    dynamic_img.save("test_files/sample_portrait.png")?;
    
    // Create a simple 300x200 RGB image (landscape)
    let mut img2 = ImageBuffer::new(300, 200);
    for (x, y, pixel) in img2.enumerate_pixels_mut() {
        let r = 255 - (x * 255 / 300) as u8;
        let g = (y * 255 / 200) as u8;
        let b = (x + y) as u8 % 255;
        *pixel = Rgb([r, g, b]);
    }
    
    let dynamic_img2 = DynamicImage::ImageRgb8(img2);
    dynamic_img2.save("test_files/sample_landscape.png")?;
    
    // Create a square image 150x150
    let mut img3 = ImageBuffer::new(150, 150);
    for (x, y, pixel) in img3.enumerate_pixels_mut() {
        let distance = ((x as i32 - 75).pow(2) + (y as i32 - 75).pow(2)) as f32;
        let intensity = (255.0 * (1.0 - distance / (75.0 * 75.0))).max(0.0) as u8;
        *pixel = Rgb([intensity, 0, 255 - intensity]);
    }
    
    let dynamic_img3 = DynamicImage::ImageRgb8(img3);
    dynamic_img3.save("test_files/sample_square.png")?;
    
    Ok(())
}