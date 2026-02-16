use hdr_oxide::image::loader::SourceImage;
use hdr_oxide::image::merge::{merge_to_hdr, merge_to_hdr_parallel, HdrImage};
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
fn test_merge_single_image() {
    // Test that single image is passed through correctly
    let img = create_test_image(10, 10, [128, 128, 128]);
    let source = SourceImage::new(PathBuf::from("test.jpg"), img, 1.0);

    let result = merge_to_hdr(&[source]).unwrap();

    assert_eq!(result.width, 10);
    assert_eq!(result.height, 10);

    // Check that pixel values are approximately preserved
    let pixel = result.data.get_pixel(0, 0);
    assert!((pixel[0] - 0.5).abs() < 0.01); // 128/255 ≈ 0.5
    assert!((pixel[1] - 0.5).abs() < 0.01);
    assert!((pixel[2] - 0.5).abs() < 0.01);
}

#[test]
fn test_merge_empty_fails() {
    // Test that empty input fails
    let result = merge_to_hdr(&[]);
    assert!(result.is_err());
}

#[test]
fn test_merge_multiple_images() {
    // Create two images with different exposures
    // Image 1: darker (shorter exposure)
    let img1 = create_test_image(4, 4, [64, 64, 64]); // Dark gray
    let source1 = SourceImage::new(PathBuf::from("img1.jpg"), img1, 1.0 / 250.0); // 1/250s

    // Image 2: brighter (longer exposure)
    let img2 = create_test_image(4, 4, [192, 192, 192]); // Light gray
    let source2 = SourceImage::new(PathBuf::from("img2.jpg"), img2, 1.0 / 60.0); // 1/60s

    let result = merge_to_hdr(&[source1, source2]).unwrap();

    assert_eq!(result.width, 4);
    assert_eq!(result.height, 4);

    // HDR result should be reasonable (not NaN or infinite)
    let pixel = result.data.get_pixel(0, 0);
    assert!(pixel[0].is_finite());
    assert!(pixel[1].is_finite());
    assert!(pixel[2].is_finite());
    assert!(pixel[0] >= 0.0);
    assert!(pixel[1] >= 0.0);
    assert!(pixel[2] >= 0.0);
}

#[test]
fn test_merge_parallel_matches_sequential() {
    // Create test images
    let img1 = create_test_image(8, 8, [100, 100, 100]);
    let source1 = SourceImage::new(PathBuf::from("img1.jpg"), img1, 1.0 / 125.0);

    let img2 = create_test_image(8, 8, [150, 150, 150]);
    let source2 = SourceImage::new(PathBuf::from("img2.jpg"), img2, 1.0 / 60.0);

    // Test both sequential and parallel produce valid results
    let sequential = merge_to_hdr(&[source1.clone(), source2.clone()]).unwrap();
    let parallel = merge_to_hdr_parallel(&[source1, source2]).unwrap();

    assert_eq!(sequential.width, parallel.width);
    assert_eq!(sequential.height, parallel.height);

    // Results should be very similar (allowing for floating point differences)
    for y in 0..sequential.height {
        for x in 0..sequential.width {
            let seq_pixel = sequential.data.get_pixel(x, y);
            let par_pixel = parallel.data.get_pixel(x, y);

            assert!((seq_pixel[0] - par_pixel[0]).abs() < 0.001);
            assert!((seq_pixel[1] - par_pixel[1]).abs() < 0.001);
            assert!((seq_pixel[2] - par_pixel[2]).abs() < 0.001);
        }
    }
}

#[test]
fn test_hdr_image_new() {
    let hdr = HdrImage::new(100, 200);

    assert_eq!(hdr.width, 100);
    assert_eq!(hdr.height, 200);
    assert_eq!(hdr.data.width(), 100);
    assert_eq!(hdr.data.height(), 200);
}

#[test]
fn test_merge_different_dimensions_fails() {
    // Create images with different dimensions
    let img1 = create_test_image(10, 10, [128, 128, 128]);
    let source1 = SourceImage::new(PathBuf::from("img1.jpg"), img1, 1.0);

    let img2 = create_test_image(20, 20, [128, 128, 128]);
    let source2 = SourceImage::new(PathBuf::from("img2.jpg"), img2, 1.0);

    let result = merge_to_hdr(&[source1, source2]);
    assert!(result.is_err());
}
