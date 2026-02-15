use crate::error::HdrError;
use crate::image::merge::HdrImage;
use image::{DynamicImage, ImageBuffer, Rgb};
use rayon::prelude::*;

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
    let (_avg_lum, max_lum) = calculate_luminance_stats(hdr);
    let white = max_lum.powi(2);

    // Collect all pixels for parallel processing
    let pixels: Vec<(u32, u32, [f32; 4])> = (0..hdr.height)
        .flat_map(|y| {
            (0..hdr.width).map(move |x| {
                let pixel = hdr.data.get_pixel(x, y);
                (x, y, [pixel[0], pixel[1], pixel[2], pixel[3]])
            })
        })
        .collect();

    // Process pixels in parallel
    let processed: Vec<(u32, u32, Rgb<u8>)> = pixels
        .par_iter()
        .map(|(x, y, pixel)| {
            let mut r = pixel[0] * settings.exposure;
            let mut g = pixel[1] * settings.exposure;
            let mut b = pixel[2] * settings.exposure;

            // Apply temperature and tint (white balance)
            (r, g, b) = apply_white_balance(r, g, b, settings.temperature, settings.tint);

            let lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;
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

            // Apply contrast
            r = apply_contrast(r, settings.contrast);
            g = apply_contrast(g, settings.contrast);
            b = apply_contrast(b, settings.contrast);

            // Apply shadows/highlights
            (r, g, b) = apply_shadows_highlights(r, g, b, settings.shadows, settings.highlights);

            // Apply hue shift
            (r, g, b) = apply_hue_shift(r, g, b, settings.hue_shift);

            // Apply saturation and vibrance
            (r, g, b) = apply_saturation_vibrance(r, g, b, settings.saturation, settings.vibrance);

            r = srgb_to_gamma(r);
            g = srgb_to_gamma(g);
            b = srgb_to_gamma(b);

            let rgb = Rgb([
                (r.clamp(0.0, 1.0) * 255.0) as u8,
                (g.clamp(0.0, 1.0) * 255.0) as u8,
                (b.clamp(0.0, 1.0) * 255.0) as u8,
            ]);
            (*x, *y, rgb)
        })
        .collect();

    // Write results back
    let mut output: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(hdr.width, hdr.height);
    for (x, y, rgb) in processed {
        output.put_pixel(x, y, rgb);
    }

    Ok(DynamicImage::ImageRgb8(output))
}

fn tonemap_filmic(hdr: &HdrImage, settings: &TonemapSettings) -> Result<DynamicImage, HdrError> {
    // Collect all pixels for parallel processing
    let pixels: Vec<(u32, u32, [f32; 4])> = (0..hdr.height)
        .flat_map(|y| {
            (0..hdr.width).map(move |x| {
                let pixel = hdr.data.get_pixel(x, y);
                (x, y, [pixel[0], pixel[1], pixel[2], pixel[3]])
            })
        })
        .collect();

    // Process pixels in parallel
    let processed: Vec<(u32, u32, Rgb<u8>)> = pixels
        .par_iter()
        .map(|(x, y, pixel)| {
            let mut r = pixel[0] * settings.exposure;
            let mut g = pixel[1] * settings.exposure;
            let mut b = pixel[2] * settings.exposure;

            // Apply temperature and tint
            (r, g, b) = apply_white_balance(r, g, b, settings.temperature, settings.tint);

            r = apply_filmic_curve(r);
            g = apply_filmic_curve(g);
            b = apply_filmic_curve(b);

            // Apply contrast
            r = apply_contrast(r, settings.contrast);
            g = apply_contrast(g, settings.contrast);
            b = apply_contrast(b, settings.contrast);

            // Apply shadows/highlights
            (r, g, b) = apply_shadows_highlights(r, g, b, settings.shadows, settings.highlights);

            // Apply hue shift
            (r, g, b) = apply_hue_shift(r, g, b, settings.hue_shift);

            // Apply saturation and vibrance
            (r, g, b) = apply_saturation_vibrance(r, g, b, settings.saturation, settings.vibrance);

            r = srgb_to_gamma(r);
            g = srgb_to_gamma(g);
            b = srgb_to_gamma(b);

            let rgb = Rgb([
                (r.clamp(0.0, 1.0) * 255.0) as u8,
                (g.clamp(0.0, 1.0) * 255.0) as u8,
                (b.clamp(0.0, 1.0) * 255.0) as u8,
            ]);
            (*x, *y, rgb)
        })
        .collect();

    // Write results back
    let mut output: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(hdr.width, hdr.height);
    for (x, y, rgb) in processed {
        output.put_pixel(x, y, rgb);
    }

    Ok(DynamicImage::ImageRgb8(output))
}

fn apply_filmic_curve(x: f32) -> f32 {
    let a = 0.15;
    let b = 0.50;
    let c = 0.10;
    let d = 0.20;
    let e = 0.02;
    let f = 0.30;

    ((x * (a * x + b * c) + d * e) / (x * (a * x + b) + d * f)) - e / f
}

fn tonemap_gamma(hdr: &HdrImage, settings: &TonemapSettings) -> Result<DynamicImage, HdrError> {
    let gamma = 1.0 / 2.2;

    // Collect all pixels for parallel processing
    let pixels: Vec<(u32, u32, [f32; 4])> = (0..hdr.height)
        .flat_map(|y| {
            (0..hdr.width).map(move |x| {
                let pixel = hdr.data.get_pixel(x, y);
                (x, y, [pixel[0], pixel[1], pixel[2], pixel[3]])
            })
        })
        .collect();

    // Process pixels in parallel
    let processed: Vec<(u32, u32, Rgb<u8>)> = pixels
        .par_iter()
        .map(|(x, y, pixel)| {
            let mut r = pixel[0] * settings.exposure;
            let mut g = pixel[1] * settings.exposure;
            let mut b = pixel[2] * settings.exposure;

            // Apply temperature and tint
            (r, g, b) = apply_white_balance(r, g, b, settings.temperature, settings.tint);

            // Apply contrast before gamma
            r = apply_contrast(r, settings.contrast);
            g = apply_contrast(g, settings.contrast);
            b = apply_contrast(b, settings.contrast);

            // Apply shadows/highlights
            (r, g, b) = apply_shadows_highlights(r, g, b, settings.shadows, settings.highlights);

            // Apply hue shift
            (r, g, b) = apply_hue_shift(r, g, b, settings.hue_shift);

            // Apply saturation and vibrance
            (r, g, b) = apply_saturation_vibrance(r, g, b, settings.saturation, settings.vibrance);

            let rgb = Rgb([
                (r.clamp(0.0, 1.0).powf(gamma) * 255.0) as u8,
                (g.clamp(0.0, 1.0).powf(gamma) * 255.0) as u8,
                (b.clamp(0.0, 1.0).powf(gamma) * 255.0) as u8,
            ]);
            (*x, *y, rgb)
        })
        .collect();

    // Write results back
    let mut output: ImageBuffer<Rgb<u8>, Vec<u8>> = ImageBuffer::new(hdr.width, hdr.height);
    for (x, y, rgb) in processed {
        output.put_pixel(x, y, rgb);
    }

    Ok(DynamicImage::ImageRgb8(output))
}

fn srgb_to_gamma(linear: f32) -> f32 {
    if linear <= 0.0031308 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

fn apply_contrast(value: f32, contrast: f32) -> f32 {
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

fn rgb_to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;

    let v = max;
    let s = if max > 0.0 { delta / max } else { 0.0 };

    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / delta + if g < b { 6.0 } else { 0.0 })
    } else if max == g {
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
        let sharpened = image.clone();
        filter3x3(&sharpened.to_rgb8(), &sharpen_kernel);
        Ok(DynamicImage::ImageRgb8(sharpened.to_rgb8()))
    } else if amount < 0.0 {
        // Blur: Gaussian blur with strength based on amount
        let blur_amount = -amount * 2.0;
        Ok(DynamicImage::ImageRgb8(blur(&image.to_rgb8(), blur_amount)))
    } else {
        Ok(image.clone())
    }
}

fn calculate_luminance_stats(hdr: &HdrImage) -> (f32, f32) {
    let mut sum_lum = 0.0f32;
    let mut max_lum = 0.0f32;
    let pixel_count = (hdr.width * hdr.height) as f32;

    for pixel in hdr.data.pixels() {
        let lum = 0.2126 * pixel[0] + 0.7152 * pixel[1] + 0.0722 * pixel[2];
        sum_lum += lum;
        max_lum = max_lum.max(lum);
    }

    let avg_lum = sum_lum / pixel_count;
    (avg_lum, max_lum)
}
