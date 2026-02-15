pub mod commands {
    use crate::cli::{CreateArgs, InfoArgs};
    use crate::error::HdrError;
    use crate::image::{alignment, loader, merge, tonemap};
    use anyhow::Result;

    pub fn create_hdr(args: CreateArgs) -> Result<()> {
        if args.verbose {
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
                .init();
        } else {
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
                .init();
        }

        log::info!("HDR-Oxide - Creating HDR image");
        log::info!("Input images: {:?}", args.input);
        log::info!("Output: {:?}", args.output);

        let exposures = args.exposure.as_deref();
        let ev_offsets = args.ev_offsets.as_deref();

        log::info!("Loading source images...");
        let images = loader::load_source_images(&args.input, exposures, ev_offsets)?;

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
                Ok(_) => {
                    log::info!("Alignment successful");
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
        println!("HDR Image Information");
        println!("=====================");
        println!("File: {:?}", args.input);

        let hdr = merge::HdrImage::from_exr(&args.input)?;
        println!("Dimensions: {} x {}", hdr.width, hdr.height);
        println!("Format: 32-bit float RGBA TIFF");

        if args.verbose {
            let mut min_values = [f32::INFINITY; 3];
            let mut max_values = [f32::NEG_INFINITY; 3];
            let mut sum_values = [0.0f32; 3];
            let pixel_count = (hdr.width * hdr.height) as f32;

            for pixel in hdr.data.pixels() {
                for i in 0..3 {
                    min_values[i] = min_values[i].min(pixel[i]);
                    max_values[i] = max_values[i].max(pixel[i]);
                    sum_values[i] += pixel[i];
                }
            }

            println!("\nChannel Statistics:");
            let channel_names = ['R', 'G', 'B'];
            for (i, name) in channel_names.iter().enumerate() {
                let avg = sum_values[i] / pixel_count;
                println!(
                    "  {}: min={:.4}, max={:.4}, avg={:.4}",
                    name, min_values[i], max_values[i], avg
                );
            }
        }

        Ok(())
    }
}
