use crate::error::HdrError;
use crate::image::merge::HdrImage;
use image::{DynamicImage, ImageBuffer, Rgb, Rgba32FImage};
use rayon::prelude::*;
use std::sync::Arc;

const LUMINANCE_R: f32 = 0.2126;
const LUMINANCE_G: f32 = 0.7152;
const LUMINANCE_B: f32 = 0.0722;
const SRGB_THRESHOLD: f32 = 0.0031308;
const SRGB_LINEAR_FACTOR: f32 = 12.92;
const SRGB_GAMMA_FACTOR: f32 = 1.055;
const SRGB_GAMMA_EXPONENT: f32 = 1.0 / 2.4;
const SRGB_GAMMA_OFFSET: f32 = 0.055;

pub struct TonemapSettings {
    pub exposure: f32,
    pub contrast: f32,
    pub saturation: f32,
    pub vibrance: f32,
    pub shadows: f32,
    pub highlights: f32,
    pub temperature: f32,
    pub tint: f32,
    pub hue_shift: f32,
    pub sharpen: f32,
}

impl Default for TonemapSettings {
    fn default() -> Self {
        Self {
            exposure: 1.0,
            contrast: 1.0,
            saturation: 1.0,
            vibrance: 0.0,
            shadows: 0.0,
            highlights: 0.0,
            temperature: 0.0,
            tint: 0.0,
            hue_shift: 0.0,
            sharpen: 0.0,
        }
    }
}

pub fn tonemap_hdr_arc(
    hdr: Arc<HdrImage>,
    method: &str,
    settings: &TonemapSettings,
) -> Result<DynamicImage, HdrError> {
    log::info!("Tonemapping HDR using {} method", method);

    let mut result = match method.to_lowercase().as_str() {
        "reinhard" => tonemap_reinhard_arc(&hdr, settings),
        "filmic" => tonemap_filmic_arc(&hdr, settings),
        "gamma" => tonemap_gamma_arc(&hdr, settings),
        _ => {
            log::warn!("Unknown tonemap method '{}', using Reinhard", method);
            tonemap_reinhard_arc(&hdr, settings)
        }
    }?;

    if settings.sharpen.abs() > 0.01 {
        result = apply_sharpen_blur(&result, settings.sharpen)?;
    }

    log::info!("Tonemapping complete");
    Ok(result)
}

fn tonemap_reinhard_arc(
    hdr: &Arc<HdrImage>,
    settings: &TonemapSettings,
) -> Result<DynamicImage, HdrError> {
    let data = hdr.data.as_ref();
    let max_lum = calculate_luminance_stats_arc(data, hdr.width, hdr.height).1;
    let white = max_lum.powi(2);

    let processed: Vec<(u32, u32, Rgb<u8>)> = (0..hdr.height)
        .into_par_iter()
        .flat_map(|y| (0..hdr.width).into_par_iter().map(move |x| (x, y)))
        .map(|(x, y)| {
            let pixel = data.get_pixel(x, y);
            let mut r = pixel[0] * settings.exposure;
            let mut g = pixel[1] * settings.exposure;
            let mut b = pixel[2] * settings.exposure;

            (r, g, b) = apply_white_balance(r, g, b, settings.temperature, settings.tint);

            let lum = LUMINANCE_R * r + LUMINANCE_G * g + LUMINANCE_B * b;
            let lum_scaled = lum * (1.0 + lum / white) / (1.0 + lum);

            if lum > 0.0 {
                let scale = lum_scaled / lum;
                r *= scale;
                g *= scale;
                b *= scale;
            }

            r = r / (r + 1.0);
            g = g / (g + 1.0);
            b = b / (b + 1.0);

            r = apply_contrast(r, settings.contrast);
            g = apply_contrast(g, settings.contrast);
            b = apply_contrast(b, settings.contrast);

            (r, g, b) = apply_shadows_highlights(r, g, b, settings.shadows, settings.highlights);
            (r, g, b) = apply_hue_shift(r, g, b, settings.hue_shift);
            (r, g, b) = apply_saturation_vibrance(r, g, b, settings.saturation, settings.vibrance);

            r = srgb_to_gamma(r);
            g = srgb_to_gamma(g);
            b = srgb_to_gamma(b);

            let rgb = Rgb([
                (r.clamp(0.0, 1.0) * 255.0) as u8,
                (g.clamp(0.0, 1.0) * 255.0) as u8,
                (b.clamp(0.0, 1.0) * 255.0) as u8,
            ]);
            (x, y, rgb)
        })
        .collect();

    let mut output: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(hdr.width, hdr.height);
    for (x, y, rgb) in processed {
        output.put_pixel(x, y, rgb);
    }

    Ok(DynamicImage::ImageRgb8(output))
}

fn tonemap_filmic_arc(
    hdr: &Arc<HdrImage>,
    settings: &TonemapSettings,
) -> Result<DynamicImage, HdrError> {
    let data = hdr.data.as_ref();

    let processed: Vec<(u32, u32, Rgb<u8>)> = (0..hdr.height)
        .into_par_iter()
        .flat_map(|y| (0..hdr.width).into_par_iter().map(move |x| (x, y)))
        .map(|(x, y)| {
            let pixel = data.get_pixel(x, y);
            let mut r = pixel[0] * settings.exposure;
            let mut g = pixel[1] * settings.exposure;
            let mut b = pixel[2] * settings.exposure;

            (r, g, b) = apply_white_balance(r, g, b, settings.temperature, settings.tint);

            r = apply_filmic_curve(r);
            g = apply_filmic_curve(g);
            b = apply_filmic_curve(b);

            r = apply_contrast(r, settings.contrast);
            g = apply_contrast(g, settings.contrast);
            b = apply_contrast(b, settings.contrast);

            (r, g, b) = apply_shadows_highlights(r, g, b, settings.shadows, settings.highlights);
            (r, g, b) = apply_hue_shift(r, g, b, settings.hue_shift);
            (r, g, b) = apply_saturation_vibrance(r, g, b, settings.saturation, settings.vibrance);

            r = srgb_to_gamma(r);
            g = srgb_to_gamma(g);
            b = srgb_to_gamma(b);

            let rgb = Rgb([
                (r.clamp(0.0, 1.0) * 255.0) as u8,
                (g.clamp(0.0, 1.0) * 255.0) as u8,
                (b.clamp(0.0, 1.0) * 255.0) as u8,
            ]);
            (x, y, rgb)
        })
        .collect();

    let mut output: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(hdr.width, hdr.height);
    for (x, y, rgb) in processed {
        output.put_pixel(x, y, rgb);
    }

    Ok(DynamicImage::ImageRgb8(output))
}

fn tonemap_gamma_arc(
    hdr: &Arc<HdrImage>,
    settings: &TonemapSettings,
) -> Result<DynamicImage, HdrError> {
    let data = hdr.data.as_ref();
    let gamma = 1.0 / 2.2;

    let processed: Vec<(u32, u32, Rgb<u8>)> = (0..hdr.height)
        .into_par_iter()
        .flat_map(|y| (0..hdr.width).into_par_iter().map(move |x| (x, y)))
        .map(|(x, y)| {
            let pixel = data.get_pixel(x, y);
            let mut r = pixel[0] * settings.exposure;
            let mut g = pixel[1] * settings.exposure;
            let mut b = pixel[2] * settings.exposure;

            (r, g, b) = apply_white_balance(r, g, b, settings.temperature, settings.tint);

            r = apply_contrast(r, settings.contrast);
            g = apply_contrast(g, settings.contrast);
            b = apply_contrast(b, settings.contrast);

            (r, g, b) = apply_shadows_highlights(r, g, b, settings.shadows, settings.highlights);
            (r, g, b) = apply_hue_shift(r, g, b, settings.hue_shift);
            (r, g, b) = apply_saturation_vibrance(r, g, b, settings.saturation, settings.vibrance);

            let rgb = Rgb([
                (r.clamp(0.0, 1.0).powf(gamma) * 255.0) as u8,
                (g.clamp(0.0, 1.0).powf(gamma) * 255.0) as u8,
                (b.clamp(0.0, 1.0).powf(gamma) * 255.0) as u8,
            ]);
            (x, y, rgb)
        })
        .collect();

    let mut output: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(hdr.width, hdr.height);
    for (x, y, rgb) in processed {
        output.put_pixel(x, y, rgb);
    }

    Ok(DynamicImage::ImageRgb8(output))
}

pub fn tonemap_hdr(
    hdr: &HdrImage,
    method: &str,
    settings: &TonemapSettings,
) -> Result<DynamicImage, HdrError> {
    log::info!("Tonemapping HDR using {} method", method);

    let mut result = match method.to_lowercase().as_str() {
        "reinhard" => tonemap_reinhard(hdr, settings),
        "filmic" => tonemap_filmic(hdr, settings),
        "gamma" => tonemap_gamma(hdr, settings),
        _ => {
            log::warn!("Unknown tonemap method '{}', using Reinhard", method);
            tonemap_reinhard(hdr, settings)
        }
    }?;

    // Apply sharpening/blur if needed
    if settings.sharpen.abs() > 0.01 {
        result = apply_sharpen_blur(&result, settings.sharpen)?;
    }

    log::info!("Tonemapping complete");
    Ok(result)
}

fn tonemap_reinhard(hdr: &HdrImage, settings: &TonemapSettings) -> Result<DynamicImage, HdrError> {
    let max_lum = calculate_luminance_stats(hdr).1;
    let white = max_lum.powi(2);

    let processed: Vec<(u32, u32, Rgb<u8>)> = (0..hdr.height)
        .into_par_iter()
        .flat_map(|y| (0..hdr.width).into_par_iter().map(move |x| (x, y)))
        .map(|(x, y)| {
            let pixel = hdr.data.get_pixel(x, y);
            let mut r = pixel[0] * settings.exposure;
            let mut g = pixel[1] * settings.exposure;
            let mut b = pixel[2] * settings.exposure;

            (r, g, b) = apply_white_balance(r, g, b, settings.temperature, settings.tint);

            let lum = LUMINANCE_R * r + LUMINANCE_G * g + LUMINANCE_B * b;
            let lum_scaled = lum * (1.0 + lum / white) / (1.0 + lum);

            if lum > 0.0 {
                let scale = lum_scaled / lum;
                r *= scale;
                g *= scale;
                b *= scale;
            }

            r = r / (r + 1.0);
            g = g / (g + 1.0);
            b = b / (b + 1.0);

            r = apply_contrast(r, settings.contrast);
            g = apply_contrast(g, settings.contrast);
            b = apply_contrast(b, settings.contrast);

            (r, g, b) = apply_shadows_highlights(r, g, b, settings.shadows, settings.highlights);
            (r, g, b) = apply_hue_shift(r, g, b, settings.hue_shift);
            (r, g, b) = apply_saturation_vibrance(r, g, b, settings.saturation, settings.vibrance);

            r = srgb_to_gamma(r);
            g = srgb_to_gamma(g);
            b = srgb_to_gamma(b);

            let rgb = Rgb([
                (r.clamp(0.0, 1.0) * 255.0) as u8,
                (g.clamp(0.0, 1.0) * 255.0) as u8,
                (b.clamp(0.0, 1.0) * 255.0) as u8,
            ]);
            (x, y, rgb)
        })
        .collect();

    let mut output: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(hdr.width, hdr.height);
    for (x, y, rgb) in processed {
        output.put_pixel(x, y, rgb);
    }

    Ok(DynamicImage::ImageRgb8(output))
}

fn tonemap_filmic(hdr: &HdrImage, settings: &TonemapSettings) -> Result<DynamicImage, HdrError> {
    let processed: Vec<(u32, u32, Rgb<u8>)> = (0..hdr.height)
        .into_par_iter()
        .flat_map(|y| (0..hdr.width).into_par_iter().map(move |x| (x, y)))
        .map(|(x, y)| {
            let pixel = hdr.data.get_pixel(x, y);
            let mut r = pixel[0] * settings.exposure;
            let mut g = pixel[1] * settings.exposure;
            let mut b = pixel[2] * settings.exposure;

            (r, g, b) = apply_white_balance(r, g, b, settings.temperature, settings.tint);

            r = apply_filmic_curve(r);
            g = apply_filmic_curve(g);
            b = apply_filmic_curve(b);

            r = apply_contrast(r, settings.contrast);
            g = apply_contrast(g, settings.contrast);
            b = apply_contrast(b, settings.contrast);

            (r, g, b) = apply_shadows_highlights(r, g, b, settings.shadows, settings.highlights);
            (r, g, b) = apply_hue_shift(r, g, b, settings.hue_shift);
            (r, g, b) = apply_saturation_vibrance(r, g, b, settings.saturation, settings.vibrance);

            r = srgb_to_gamma(r);
            g = srgb_to_gamma(g);
            b = srgb_to_gamma(b);

            let rgb = Rgb([
                (r.clamp(0.0, 1.0) * 255.0) as u8,
                (g.clamp(0.0, 1.0) * 255.0) as u8,
                (b.clamp(0.0, 1.0) * 255.0) as u8,
            ]);
            (x, y, rgb)
        })
        .collect();

    let mut output: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(hdr.width, hdr.height);
    for (x, y, rgb) in processed {
        output.put_pixel(x, y, rgb);
    }

    Ok(DynamicImage::ImageRgb8(output))
}

const FILMIC_A: f32 = 0.15;
const FILMIC_B: f32 = 0.50;
const FILMIC_C: f32 = 0.10;
const FILMIC_D: f32 = 0.20;
const FILMIC_E: f32 = 0.02;
const FILMIC_F: f32 = 0.30;

fn apply_filmic_curve(x: f32) -> f32 {
    let x = x.max(0.0);
    let denominator = x * (FILMIC_A * x + FILMIC_B) + FILMIC_D * FILMIC_F;
    if denominator.abs() < 1e-10 {
        return 0.0;
    }
    let result = ((x * (FILMIC_A * x + FILMIC_B * FILMIC_C) + FILMIC_D * FILMIC_E) / denominator)
        - FILMIC_E / FILMIC_F;
    if result.is_nan() {
        0.0
    } else {
        result
    }
}

fn tonemap_gamma(hdr: &HdrImage, settings: &TonemapSettings) -> Result<DynamicImage, HdrError> {
    let gamma = 1.0 / 2.2;

    let processed: Vec<(u32, u32, Rgb<u8>)> = (0..hdr.height)
        .into_par_iter()
        .flat_map(|y| (0..hdr.width).into_par_iter().map(move |x| (x, y)))
        .map(|(x, y)| {
            let pixel = hdr.data.get_pixel(x, y);
            let mut r = pixel[0] * settings.exposure;
            let mut g = pixel[1] * settings.exposure;
            let mut b = pixel[2] * settings.exposure;

            (r, g, b) = apply_white_balance(r, g, b, settings.temperature, settings.tint);

            r = apply_contrast(r, settings.contrast);
            g = apply_contrast(g, settings.contrast);
            b = apply_contrast(b, settings.contrast);

            (r, g, b) = apply_shadows_highlights(r, g, b, settings.shadows, settings.highlights);
            (r, g, b) = apply_hue_shift(r, g, b, settings.hue_shift);
            (r, g, b) = apply_saturation_vibrance(r, g, b, settings.saturation, settings.vibrance);

            let rgb = Rgb([
                (r.clamp(0.0, 1.0).powf(gamma) * 255.0) as u8,
                (g.clamp(0.0, 1.0).powf(gamma) * 255.0) as u8,
                (b.clamp(0.0, 1.0).powf(gamma) * 255.0) as u8,
            ]);
            (x, y, rgb)
        })
        .collect();

    let mut output: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(hdr.width, hdr.height);
    for (x, y, rgb) in processed {
        output.put_pixel(x, y, rgb);
    }

    Ok(DynamicImage::ImageRgb8(output))
}

fn srgb_to_gamma(linear: f32) -> f32 {
    if linear <= SRGB_THRESHOLD {
        linear * SRGB_LINEAR_FACTOR
    } else {
        SRGB_GAMMA_FACTOR * linear.powf(SRGB_GAMMA_EXPONENT) - SRGB_GAMMA_OFFSET
    }
}

fn apply_contrast(value: f32, contrast: f32) -> f32 {
    let contrast = contrast.max(0.01);
    0.5 + (value - 0.5) * contrast
}

fn apply_white_balance(r: f32, g: f32, b: f32, temp: f32, tint: f32) -> (f32, f32, f32) {
    // Temperature: shift toward blue (negative) or orange (positive)
    // Simplified: adjust red and blue channels
    let temp_factor = temp / 100.0; // Scale to usable range
    let r_adj = r * (1.0 + temp_factor * 0.1);
    let b_adj = b * (1.0 - temp_factor * 0.1);

    // Tint: shift toward green (negative) or magenta (positive)
    let tint_factor = tint / 100.0;
    let g_adj = g * (1.0 + tint_factor * 0.05);
    let r_tint = r_adj * (1.0 - tint_factor * 0.05);
    let b_tint = b_adj * (1.0 - tint_factor * 0.05);

    (r_tint, g_adj, b_tint)
}

fn apply_shadows_highlights(
    r: f32,
    g: f32,
    b: f32,
    shadows: f32,
    highlights: f32,
) -> (f32, f32, f32) {
    let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;

    // Shadows: brighten dark areas
    let shadow_lift = if lum < 0.5 {
        shadows * (0.5 - lum) * 0.5
    } else {
        0.0
    };

    // Highlights: darken bright areas
    let highlight_compress = if lum > 0.5 {
        highlights * (lum - 0.5) * 0.5
    } else {
        0.0
    };

    let factor = 1.0 + shadow_lift - highlight_compress;
    (
        (r * factor).clamp(0.0, 1.0),
        (g * factor).clamp(0.0, 1.0),
        (b * factor).clamp(0.0, 1.0),
    )
}

const EPSILON: f32 = 1e-6;

fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let v = max;
    let s = if max > EPSILON { delta / max } else { 0.0 };

    let h = if delta < EPSILON {
        0.0
    } else if (max - r).abs() < EPSILON {
        60.0 * ((g - b) / delta + if g < b { 6.0 } else { 0.0 })
    } else if (max - g).abs() < EPSILON {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };

    (h / 360.0, s, v)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let h = h * 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    (r1 + m, g1 + m, b1 + m)
}

fn apply_hue_shift(r: f32, g: f32, b: f32, hue_shift: f32) -> (f32, f32, f32) {
    if hue_shift == 0.0 {
        return (r, g, b);
    }

    let (h, s, v) = rgb_to_hsv(r, g, b);
    let new_h = (h + hue_shift).rem_euclid(1.0);
    hsv_to_rgb(new_h, s, v)
}

fn apply_saturation_vibrance(
    r: f32,
    g: f32,
    b: f32,
    saturation: f32,
    vibrance: f32,
) -> (f32, f32, f32) {
    let (h, s, v) = rgb_to_hsv(r, g, b);

    // Vibrance increases saturation more for less saturated colors (protects skin tones)
    let vibrance_boost = if vibrance > 0.0 {
        vibrance * (1.0 - s) * 0.5
    } else {
        vibrance * s * 0.5
    };

    let new_s = (s * saturation + vibrance_boost).clamp(0.0, 1.0);
    hsv_to_rgb(h, new_s, v)
}

fn apply_sharpen_blur(image: &DynamicImage, amount: f32) -> Result<DynamicImage, HdrError> {
    use image::imageops::{blur, filter3x3};

    if amount > 0.0 {
        // Sharpen: apply unsharp mask using a 3x3 kernel
        let sharpen_kernel: [f32; 9] = [
            0.0,
            -amount,
            0.0,
            -amount,
            1.0 + 4.0 * amount,
            -amount,
            0.0,
            -amount,
            0.0,
        ];
        let rgb_image = image.to_rgb8();
        let sharpened = filter3x3(&rgb_image, &sharpen_kernel);
        Ok(DynamicImage::ImageRgb8(sharpened))
    } else if amount < 0.0 {
        // Blur: Gaussian blur with strength based on amount
        let blur_amount = -amount * 2.0;
        Ok(DynamicImage::ImageRgb8(blur(&image.to_rgb8(), blur_amount)))
    } else {
        Ok(image.clone())
    }
}

fn calculate_luminance_stats(hdr: &HdrImage) -> (f32, f32) {
    let data = hdr.data.as_ref();
    let mut sum_lum = 0.0f32;
    let mut max_lum = 0.0f32;
    let pixel_count = (hdr.width as u64 * hdr.height as u64) as f32;

    for pixel in data.pixels() {
        let lum = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
        sum_lum += lum;
        max_lum = max_lum.max(lum);
    }

    let avg_lum = sum_lum / pixel_count;
    (avg_lum, max_lum)
}

fn calculate_luminance_stats_arc(data: &Rgba32FImage, width: u32, height: u32) -> (f32, f32) {
    let mut sum_lum = 0.0f32;
    let mut max_lum = 0.0f32;
    let pixel_count = (width as u64 * height as u64) as f32;

    for pixel in data.pixels() {
        let lum = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
        sum_lum += lum;
        max_lum = max_lum.max(lum);
    }

    let avg_lum = sum_lum / pixel_count;
    (avg_lum, max_lum)
}
