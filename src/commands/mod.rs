use crate::cli::{CreateArgs, InfoArgs};
use crate::error::HdrError;
use crate::image::{alignment, loader, merge, tonemap};
use anyhow::Result;
use image::GenericImageView;

pub fn create_hdr(args: CreateArgs) -> Result<()> {
    // Try to init logger, but ignore error if already initialized
    let _ = if args.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .try_init()
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
            .try_init()
    };

    log::info!("HDR-Oxide - Creating HDR image");
    log::info!("Input images: {:?}", args.input);
    log::info!("Output: {:?}", args.output);

    let exposures = args.exposure.as_deref();
    let ev_offsets = args.ev_offsets.as_deref();

    log::info!("Loading source images...");
    let mut images = loader::load_source_images(&args.input, exposures, ev_offsets)?;

    if images.len() < 2 {
        return Err(
            HdrError::InvalidInput("Need at least 2 images to create HDR".to_string()).into(),
        );
    }

    if !args.no_align {
        log::info!("Attempting image alignment (for handheld shots)...");
        let mut img_list: Vec<(std::path::PathBuf, image::DynamicImage)> = images
            .iter()
            .map(|img| (img.path.clone(), img.image.clone()))
            .collect();

        match alignment::align_images(&mut img_list) {
            Ok(()) => {
                log::info!("Alignment successful");
                // Update the source images with aligned versions
                for (i, (_, aligned_img)) in img_list.into_iter().enumerate() {
                    images[i].image = aligned_img;
                    // Update dimensions in case alignment changed them
                    let (new_width, new_height) = images[i].image.dimensions();
                    images[i].width = new_width;
                    images[i].height = new_height;
                }
            }
            Err(e) => {
                log::warn!("Alignment failed: {}", e);
                log::info!("For tripod/bracketed HDR shots, use --no-align (recommended)");
            }
        }
    } else {
        log::info!("Skipping alignment (--no-align flag)");
    }

    log::info!("Merging images to HDR...");
    let pixel_count = images[0].width * images[0].height;
    let hdr = if pixel_count > 500_000 {
        log::info!(
            "Using parallel merge for large image ({}x{} = {} pixels)",
            images[0].width,
            images[0].height,
            pixel_count
        );
        merge::merge_to_hdr_parallel(&images)?
    } else {
        merge::merge_to_hdr(&images)?
    };

    log::info!("Tonemapping HDR...");
    let settings = tonemap::TonemapSettings {
        exposure: args.exposure_adjust,
        contrast: args.contrast,
        saturation: args.saturation,
        vibrance: args.vibrance / 100.0,
        shadows: args.shadows / 100.0,
        highlights: args.highlights / 100.0,
        temperature: args.temperature,
        tint: args.tint,
        hue_shift: args.hue_shift / 360.0,
        sharpen: args.sharpen / 100.0,
    };
    let tonemapped = tonemap::tonemap_hdr(&hdr, &args.tonemap_method, &settings)?;

    log::info!("Saving to {:?}", args.output);
    tonemapped.save(&args.output)?;

    log::info!("HDR creation complete!");
    Ok(())
}

pub fn info_hdr(args: InfoArgs) -> Result<()> {
    println!("Image Information");
    println!("=================");
    println!("File: {:?}", args.input);

    // Check file exists
    if !args.input.exists() {
        return Err(HdrError::Io(format!("File not found: {:?}", args.input)).into());
    }

    // Get file metadata
    let metadata = std::fs::metadata(&args.input)?;
    println!("Size: {} bytes", metadata.len());

    // Try to open with image crate
    match image::open(&args.input) {
        Ok(img) => {
            let (width, height) = img.dimensions();
            println!("Dimensions: {} x {}", width, height);

            // Determine format
            let format = match img {
                image::DynamicImage::ImageRgb8(_) => "8-bit RGB",
                image::DynamicImage::ImageRgba8(_) => "8-bit RGBA",
                image::DynamicImage::ImageRgb16(_) => "16-bit RGB",
                image::DynamicImage::ImageRgba16(_) => "16-bit RGBA",
                image::DynamicImage::ImageRgb32F(_) => "32-bit float RGB",
                image::DynamicImage::ImageRgba32F(_) => "32-bit float RGBA",
                image::DynamicImage::ImageLuma8(_) => "8-bit Grayscale",
                image::DynamicImage::ImageLumaA8(_) => "8-bit Grayscale+Alpha",
                image::DynamicImage::ImageLuma16(_) => "16-bit Grayscale",
                image::DynamicImage::ImageLumaA16(_) => "16-bit Grayscale+Alpha",
                _ => "Unknown",
            };
            println!("Format: {}", format);

            if args.verbose {
                println!("\nNote: Detailed channel statistics not yet implemented");
            }
        }
        Err(e) => {
            println!("Could not read image: {}", e);
        }
    }

    Ok(())
}
