use hdr_oxide::image::loader::{parse_exposure_string, SourceImage};
use image::{DynamicImage, Rgb, RgbImage};
use std::path::PathBuf;

fn create_test_image(width: u32, height: u32, color: [u8; 3]) -> DynamicImage {
    let mut img = RgbImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            img.put_pixel(x, y, Rgb(color));
        }
    }
    DynamicImage::ImageRgb8(img)
}

#[test]
fn test_parse_exposure_string_decimal() {
    // Test decimal format
    let result = parse_exposure_string("1.0").unwrap();
    assert!((result - 1.0).abs() < 0.001);

    let result = parse_exposure_string("0.5").unwrap();
    assert!((result - 0.5).abs() < 0.001);

    let result = parse_exposure_string("2.0").unwrap();
    assert!((result - 2.0).abs() < 0.001);

    let result = parse_exposure_string("0.008").unwrap();
    assert!((result - 0.008).abs() < 0.0001);
}

#[test]
fn test_parse_exposure_string_fraction() {
    // Test fraction format (common in photography)
    let result = parse_exposure_string("1/125").unwrap();
    assert!((result - 0.008).abs() < 0.0001);

    let result = parse_exposure_string("1/60").unwrap();
    assert!((result - 0.016666).abs() < 0.0001);

    let result = parse_exposure_string("1/1000").unwrap();
    assert!((result - 0.001).abs() < 0.0001);

    let result = parse_exposure_string("1/2").unwrap();
    assert!((result - 0.5).abs() < 0.001);
}

#[test]
fn test_parse_exposure_string_invalid() {
    // Test invalid inputs
    assert!(parse_exposure_string("abc").is_err());
    assert!(parse_exposure_string("").is_err());
    assert!(parse_exposure_string("1/0").is_err()); // Division by zero
    assert!(parse_exposure_string("1/x").is_err());
}

#[test]
fn test_source_image_creation() {
    let img = create_test_image(100, 200, [128, 128, 128]);
    let path = PathBuf::from("test.jpg");
    let exposure = 1.0 / 125.0;

    let source = SourceImage::new(path.clone(), img, exposure);

    assert_eq!(source.path, path);
    assert_eq!(source.width, 100);
    assert_eq!(source.height, 200);
    assert!((source.exposure_seconds - exposure).abs() < 0.0001);
}

#[test]
fn test_source_image_to_rgb32f() {
    // Create a test image with known color values
    let img = create_test_image(2, 2, [255, 128, 64]);
    let source = SourceImage::new(PathBuf::from("test.jpg"), img, 1.0);

    let rgb32f = source.to_rgb32f();

    // Check dimensions
    assert_eq!(rgb32f.width(), 2);
    assert_eq!(rgb32f.height(), 2);

    // Check pixel values (normalized to 0.0-1.0 range)
    let pixel = rgb32f.get_pixel(0, 0);
    assert!((pixel[0] - 1.0).abs() < 0.01); // 255/255 = 1.0
    assert!((pixel[1] - 0.5).abs() < 0.01); // 128/255 ≈ 0.5
    assert!((pixel[2] - 0.25).abs() < 0.01); // 64/255 ≈ 0.25
}
