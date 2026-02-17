use crate::error::HdrError;
use crate::image::loader::SourceImage;
use image::{ImageBuffer, Rgba, Rgba32FImage};
use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub struct HdrImage {
    pub data: Arc<Rgba32FImage>,
    pub width: u32,
    pub height: u32,
    pub max_luminance: f32,
}

impl HdrImage {
    pub fn new(width: u32, height: u32) -> Self {
        let data = ImageBuffer::new(width, height);
        Self {
            data: Arc::new(data),
            width,
            height,
            max_luminance: 0.0,
        }
    }

    pub fn from_exr(_path: &std::path::Path) -> std::result::Result<Self, HdrError> {
        Err(HdrError::Exr("EXR reading not yet implemented".to_string()))
    }

    fn calculate_max_luminance(data: &Rgba32FImage) -> f32 {
        let mut max_lum = 0.0f32;
        for pixel in data.pixels() {
            let lum = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
            max_lum = max_lum.max(lum);
        }
        max_lum
    }
}

pub fn merge_to_hdr(images: &[SourceImage]) -> std::result::Result<HdrImage, HdrError> {
    if images.is_empty() {
        return Err(HdrError::Merge("No images to merge".to_string()));
    }

    if images.len() == 1 {
        let img = &images[0];
        let rgb = img.image.to_rgb32f();
        let mut rgba: Rgba32FImage = ImageBuffer::new(img.width, img.height);

        for y in 0..img.height {
            for x in 0..img.width {
                let pixel = rgb.get_pixel(x, y);
                rgba.put_pixel(x, y, Rgba([pixel[0], pixel[1], pixel[2], 1.0]));
            }
        }

        let max_lum = HdrImage::calculate_max_luminance(&rgba);
        return Ok(HdrImage {
            data: Arc::new(rgba),
            width: img.width,
            height: img.height,
            max_luminance: max_lum,
        });
    }

    let width = images[0].width;
    let height = images[0].height;

    for (i, img) in images.iter().enumerate().skip(1) {
        if img.width != width || img.height != height {
            return Err(HdrError::Merge(format!(
                "Image {} has different dimensions ({}x{}) than first image ({}x{})",
                i, img.width, img.height, width, height
            )));
        }
    }

    log::info!(
        "Merging {} images into HDR ({}x{})",
        images.len(),
        width,
        height
    );

    let exposures: Vec<f32> = images.iter().map(|i| i.exposure_seconds).collect();

    let img_data: Vec<image::Rgb32FImage> =
        images.iter().map(|img| img.image.to_rgb32f()).collect();

    let mut hdr_data: Rgba32FImage = ImageBuffer::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let mut r_sum = 0.0f32;
            let mut g_sum = 0.0f32;
            let mut b_sum = 0.0f32;
            let mut weight_sum = 0.0f32;

            for (img_idx, rgb) in img_data.iter().enumerate() {
                let pixel = rgb.get_pixel(x, y);

                let orig_r = pixel[0];
                let orig_g = pixel[1];
                let orig_b = pixel[2];

                let r = orig_r / exposures[img_idx];
                let g = orig_g / exposures[img_idx];
                let b = orig_b / exposures[img_idx];

                let weight = calculate_weight(orig_r, orig_g, orig_b);

                r_sum += r * weight;
                g_sum += g * weight;
                b_sum += b * weight;
                weight_sum += weight;
            }

            let (r, g, b) = if weight_sum > 0.0 {
                (r_sum / weight_sum, g_sum / weight_sum, b_sum / weight_sum)
            } else {
                log::warn!(
                    "Pixel at ({}, {}) has zero total weight, using fallback",
                    x,
                    y
                );
                (0.0f32, 0.0f32, 0.0f32)
            };

            hdr_data.put_pixel(x, y, Rgba([r, g, b, 1.0]));
        }
    }

    log::info!("HDR merge complete");

    let max_lum = HdrImage::calculate_max_luminance(&hdr_data);
    Ok(HdrImage {
        data: Arc::new(hdr_data),
        width,
        height,
        max_luminance: max_lum,
    })
}

fn calculate_weight(r: f32, g: f32, b: f32) -> f32 {
    // Standard Debevec weight: prefer middle exposures, avoid extremes
    let z = r.max(g).max(b);

    if z <= 0.0 || z >= 1.0 {
        return 0.0;
    }

    // Weight curve that peaks at 0.5 and falls off toward 0 and 1
    // Using a smoother curve than triangular
    1.0 - (2.0 * z - 1.0).abs()
}

pub fn merge_to_hdr_parallel(images: &[SourceImage]) -> std::result::Result<HdrImage, HdrError> {
    merge_to_hdr_parallel_with_progress(images, |_| {})
}

pub fn merge_to_hdr_parallel_with_progress<F>(
    images: &[SourceImage],
    progress_callback: F,
) -> std::result::Result<HdrImage, HdrError>
where
    F: Fn(usize) + Send + Sync,
{
    if images.is_empty() {
        return Err(HdrError::Merge("No images to merge".to_string()));
    }

    let width = images[0].width;
    let height = images[0].height;

    for (i, img) in images.iter().enumerate().skip(1) {
        if img.width != width || img.height != height {
            return Err(HdrError::Merge(format!(
                "Image {} has different dimensions ({}x{}) than first image ({}x{})",
                i, img.width, img.height, width, height
            )));
        }
    }

    log::info!(
        "Merging {} images into HDR ({}x{}) using parallel processing",
        images.len(),
        width,
        height
    );

    let img_data: Vec<image::Rgb32FImage> =
        images.iter().map(|img| img.image.to_rgb32f()).collect();

    let exposures: Vec<f32> = images.iter().map(|i| i.exposure_seconds).collect();

    let mut hdr_data: Rgba32FImage = ImageBuffer::new(width, height);
    let total_pixels = height as usize;
    let processed = Arc::new(AtomicUsize::new(0));
    let last_reported = Arc::new(AtomicUsize::new(0));
    let progress_callback = Arc::new(progress_callback);

    let results: Vec<(u32, u32, f32, f32, f32)> = (0..height)
        .into_par_iter()
        .flat_map(|y| (0..width).into_par_iter().map(move |x| (x, y)))
        .map(|(x, y)| {
            let mut r_sum = 0.0f32;
            let mut g_sum = 0.0f32;
            let mut b_sum = 0.0f32;
            let mut weight_sum = 0.0f32;

            for (img_idx, rgb) in img_data.iter().enumerate() {
                let pixel = rgb.get_pixel(x, y);

                let orig_r = pixel[0];
                let orig_g = pixel[1];
                let orig_b = pixel[2];

                let r = orig_r / exposures[img_idx];
                let g = orig_g / exposures[img_idx];
                let b = orig_b / exposures[img_idx];

                let weight = calculate_weight(orig_r, orig_g, orig_b);

                r_sum += r * weight;
                g_sum += g * weight;
                b_sum += b * weight;
                weight_sum += weight;
            }

            let (r, g, b) = if weight_sum > 0.0 {
                (r_sum / weight_sum, g_sum / weight_sum, b_sum / weight_sum)
            } else {
                (0.0f32, 0.0f32, 0.0f32)
            };

            let count = processed.fetch_add(1, Ordering::Relaxed);
            if count.saturating_sub(last_reported.load(Ordering::Relaxed)) >= total_pixels / 100 {
                last_reported.store(count, Ordering::Relaxed);
                progress_callback(count);
            }

            (x, y, r, g, b)
        })
        .collect();

    progress_callback(total_pixels);

    for (x, y, r, g, b) in results {
        hdr_data.put_pixel(x, y, Rgba([r, g, b, 1.0]));
    }

    log::info!("HDR merge complete (parallel)");

    let max_lum = HdrImage::calculate_max_luminance(&hdr_data);
    Ok(HdrImage {
        data: Arc::new(hdr_data),
        width,
        height,
        max_luminance: max_lum,
    })
}
