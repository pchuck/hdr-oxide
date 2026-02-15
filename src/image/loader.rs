use crate::error::HdrError;
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgb, Rgb32FImage};
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SourceImage {
    pub path: PathBuf,
    pub image: DynamicImage,
    pub exposure_seconds: f32,
    pub width: u32,
    pub height: u32,
}

impl SourceImage {
    pub fn new(path: PathBuf, image: DynamicImage, exposure_seconds: f32) -> Self {
        let (width, height) = image.dimensions();
        Self {
            path,
            image,
            exposure_seconds,
            width,
            height,
        }
    }

    pub fn to_rgb32f(&self) -> Rgb32FImage {
        let rgba = self.image.to_rgba8();
        let (width, height) = (self.width, self.height);

        let mut rgb_f32: Rgb32FImage = ImageBuffer::new(width, height);

        for y in 0..height {
            for x in 0..width {
                let pixel = rgba.get_pixel(x, y);
                let r = pixel[0] as f32 / 255.0;
                let g = pixel[1] as f32 / 255.0;
                let b = pixel[2] as f32 / 255.0;
                rgb_f32.put_pixel(x, y, Rgb([r, g, b]));
            }
        }

        rgb_f32
    }
}

pub fn load_image(path: &Path) -> Result<DynamicImage, HdrError> {
    let img = image::open(path)?;
    Ok(img)
}

pub fn extract_exposure_time(path: &Path) -> Result<f32, HdrError> {
    let file = File::open(path)?;
    let mut bufreader = BufReader::new(file);
    let exifreader = exif::Reader::new();

    match exifreader.read_from_container(&mut bufreader) {
        Ok(exif) => {
            if let Some(field) = exif.get_field(exif::Tag::ExposureTime, exif::In::PRIMARY) {
                if let exif::Value::Rational(ref v) = field.value {
                    if !v.is_empty() {
                        let exposure = v[0].to_f64() as f32;
                        return Ok(exposure);
                    }
                }
            }
            Err(HdrError::Exif(
                "Could not find exposure time in EXIF data".to_string(),
            ))
        }
        Err(_) => Err(HdrError::Exif("Could not read EXIF data".to_string())),
    }
}

pub fn parse_exposure_string(s: &str) -> Result<f32, HdrError> {
    if let Some((num, denom)) = s.split_once('/') {
        let numerator: f32 = num
            .parse()
            .map_err(|_| HdrError::InvalidInput(format!("Invalid exposure: {}", s)))?;
        let denominator: f32 = denom
            .parse()
            .map_err(|_| HdrError::InvalidInput(format!("Invalid exposure: {}", s)))?;
        if denominator == 0.0 {
            return Err(HdrError::InvalidInput(
                "Division by zero in exposure".to_string(),
            ));
        }
        Ok(numerator / denominator)
    } else {
        s.parse::<f32>()
            .map_err(|_| HdrError::InvalidInput(format!("Invalid exposure: {}", s)))
    }
}

pub fn load_source_images(
    paths: &[PathBuf],
    exposures: Option<&[String]>,
    ev_offsets: Option<&[i32]>,
) -> Result<Vec<SourceImage>, HdrError> {
    if paths.is_empty() {
        return Err(HdrError::InvalidInput(
            "No input images provided".to_string(),
        ));
    }

    let mut images = Vec::with_capacity(paths.len());

    for (i, path) in paths.iter().enumerate() {
        log::info!("Loading image: {:?}", path);

        let img = load_image(path)?;
        let (width, height) = img.dimensions();

        let exposure = if let Some(exp_list) = exposures {
            if i < exp_list.len() {
                parse_exposure_string(&exp_list[i])?
            } else {
                return Err(HdrError::InvalidInput(format!(
                    "Exposure not provided for image {} (index {})",
                    path.display(),
                    i
                )));
            }
        } else if let Some(offsets) = ev_offsets {
            let ev_offset = offsets.get(i).copied().unwrap_or(0);
            let base_exposure = extract_exposure_time(path)?;
            base_exposure * 2.0_f32.powi(ev_offset)
        } else {
            match extract_exposure_time(path) {
                Ok(exp) => exp,
                Err(_) => {
                    log::warn!("Could not read exposure from {:?}, assuming 1/125", path);
                    1.0 / 125.0
                }
            }
        };

        log::info!(
            "  Dimensions: {}x{}, Exposure: {}s",
            width,
            height,
            exposure
        );

        images.push(SourceImage::new(path.clone(), img, exposure));
    }

    Ok(images)
}
