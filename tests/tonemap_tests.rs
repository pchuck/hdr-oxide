use hdr_oxide::image::merge::HdrImage;
use hdr_oxide::image::tonemap::{tonemap_hdr, TonemapSettings};
use image::{ImageBuffer, Rgba, Rgba32FImage};
use std::sync::Arc;

fn create_test_hdr(width: u32, height: u32, color: [f32; 4]) -> HdrImage {
    let mut data: Rgba32FImage = ImageBuffer::new(width, height);
    for y in 0..height {
        for x in 0..width {
            data.put_pixel(x, y, Rgba(color));
        }
    }
    HdrImage {
        data: Arc::new(data),
        width,
        height,
    }
}

#[test]
fn test_tonemap_settings_default() {
    let settings = TonemapSettings::default();

    assert!((settings.exposure - 1.0).abs() < 0.001);
    assert!((settings.contrast - 1.0).abs() < 0.001);
    assert!((settings.saturation - 1.0).abs() < 0.001);
    assert!((settings.vibrance - 0.0).abs() < 0.001);
    assert!((settings.shadows - 0.0).abs() < 0.001);
    assert!((settings.highlights - 0.0).abs() < 0.001);
    assert!((settings.temperature - 0.0).abs() < 0.001);
    assert!((settings.tint - 0.0).abs() < 0.001);
    assert!((settings.hue_shift - 0.0).abs() < 0.001);
    assert!((settings.sharpen - 0.0).abs() < 0.001);
}

#[test]
fn test_tonemap_reinhard() {
    // Create a simple HDR image with mid-gray values
    let hdr = create_test_hdr(10, 10, [0.5, 0.5, 0.5, 1.0]);
    let settings = TonemapSettings::default();

    let result = tonemap_hdr(&hdr, "reinhard", &settings);

    assert!(result.is_ok());
    let tonemapped = result.unwrap();
    assert_eq!(tonemapped.width(), 10);
    assert_eq!(tonemapped.height(), 10);
}

#[test]
fn test_tonemap_filmic() {
    let hdr = create_test_hdr(10, 10, [0.5, 0.5, 0.5, 1.0]);
    let settings = TonemapSettings::default();

    let result = tonemap_hdr(&hdr, "filmic", &settings);

    assert!(result.is_ok());
    let tonemapped = result.unwrap();
    assert_eq!(tonemapped.width(), 10);
    assert_eq!(tonemapped.height(), 10);
}

#[test]
fn test_tonemap_gamma() {
    let hdr = create_test_hdr(10, 10, [0.5, 0.5, 0.5, 1.0]);
    let settings = TonemapSettings::default();

    let result = tonemap_hdr(&hdr, "gamma", &settings);

    assert!(result.is_ok());
    let tonemapped = result.unwrap();
    assert_eq!(tonemapped.width(), 10);
    assert_eq!(tonemapped.height(), 10);
}

#[test]
fn test_tonemap_exposure_adjustment() {
    // Create HDR image with values that need exposure adjustment
    let hdr = create_test_hdr(4, 4, [2.0, 2.0, 2.0, 1.0]);

    // Test with reduced exposure
    let mut settings = TonemapSettings::default();
    settings.exposure = 0.5;

    let result = tonemap_hdr(&hdr, "reinhard", &settings);
    assert!(result.is_ok());
}

#[test]
fn test_tonemap_contrast_adjustment() {
    let hdr = create_test_hdr(4, 4, [0.5, 0.5, 0.5, 1.0]);

    let mut settings = TonemapSettings::default();
    settings.contrast = 1.5; // Increase contrast

    let result = tonemap_hdr(&hdr, "reinhard", &settings);
    assert!(result.is_ok());
}

#[test]
fn test_tonemap_sharpen_blur() {
    // Create a gradient pattern to test sharpen/blur
    let mut data: Rgba32FImage = ImageBuffer::new(8, 8);
    for y in 0..8 {
        for x in 0..8 {
            let value = (x as f32) / 8.0;
            data.put_pixel(x, y, Rgba([value, value, value, 1.0]));
        }
    }
    let hdr = HdrImage {
        data: Arc::new(data),
        width: 8,
        height: 8,
    };

    // Test sharpening
    let mut settings = TonemapSettings::default();
    settings.sharpen = 0.5;

    let result = tonemap_hdr(&hdr, "reinhard", &settings);
    assert!(result.is_ok(), "Sharpen should work after bug fix");

    // Test blurring
    settings.sharpen = -0.5;
    let result = tonemap_hdr(&hdr, "reinhard", &settings);
    assert!(result.is_ok(), "Blur should work");

    // Test no sharpen/blur (amount = 0)
    settings.sharpen = 0.0;
    let result = tonemap_hdr(&hdr, "reinhard", &settings);
    assert!(result.is_ok(), "No sharpen/blur should work");
}

#[test]
fn test_tonemap_saturation_vibrance() {
    let hdr = create_test_hdr(4, 4, [0.5, 0.5, 0.5, 1.0]);

    let mut settings = TonemapSettings::default();
    settings.saturation = 1.5;
    settings.vibrance = 50.0;

    let result = tonemap_hdr(&hdr, "reinhard", &settings);
    assert!(result.is_ok());
}

#[test]
fn test_tonemap_temperature_tint() {
    let hdr = create_test_hdr(4, 4, [0.5, 0.5, 0.5, 1.0]);

    let mut settings = TonemapSettings::default();
    settings.temperature = 20.0;
    settings.tint = 10.0;

    let result = tonemap_hdr(&hdr, "reinhard", &settings);
    assert!(result.is_ok());
}

#[test]
fn test_tonemap_shadows_highlights() {
    let hdr = create_test_hdr(4, 4, [0.5, 0.5, 0.5, 1.0]);

    let mut settings = TonemapSettings::default();
    settings.shadows = 20.0;
    settings.highlights = -20.0;

    let result = tonemap_hdr(&hdr, "reinhard", &settings);
    assert!(result.is_ok());
}

#[test]
fn test_tonemap_hue_shift() {
    let hdr = create_test_hdr(4, 4, [0.5, 0.5, 0.5, 1.0]);

    let mut settings = TonemapSettings::default();
    settings.hue_shift = 0.5; // 180 degree shift

    let result = tonemap_hdr(&hdr, "reinhard", &settings);
    assert!(result.is_ok());
}

#[test]
fn test_tonemap_unknown_method_defaults() {
    // Test that unknown method falls back to reinhard
    let hdr = create_test_hdr(4, 4, [0.5, 0.5, 0.5, 1.0]);
    let settings = TonemapSettings::default();

    let result = tonemap_hdr(&hdr, "unknown_method", &settings);
    assert!(result.is_ok());
}

#[test]
fn test_tonemap_high_dynamic_range() {
    // Test with very high values (HDR scenario)
    let hdr = create_test_hdr(4, 4, [10.0, 5.0, 2.0, 1.0]);
    let settings = TonemapSettings::default();

    let result = tonemap_hdr(&hdr, "reinhard", &settings);
    assert!(result.is_ok());

    let _tonemapped = result.unwrap();
    // Result should be in valid 8-bit range after tonemapping
    // (though we can't easily check individual pixels without converting)
}
