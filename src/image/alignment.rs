use crate::error::HdrError;
use image::DynamicImage;
use std::path::PathBuf;

#[cfg(feature = "alignment")]
pub fn align_images(images: &mut Vec<(PathBuf, DynamicImage)>) -> Result<(), HdrError> {
    if images.len() < 2 {
        return Err(HdrError::Alignment(
            "Need at least 2 images for alignment".to_string(),
        ));
    }

    log::info!(
        "Aligning {} images using ORB feature matching",
        images.len()
    );

    match align_with_orb(images) {
        Ok(aligned_imgs) => {
            log::info!("Alignment complete");
            // Update the input vector with aligned images
            for (i, aligned_img) in aligned_imgs.into_iter().enumerate() {
                images[i].1 = aligned_img;
            }
            Ok(())
        }
        Err(e) => {
            log::warn!("Alignment failed: {}", e);
            Err(HdrError::Alignment(format!(
                "Alignment not possible: {}",
                e
            )))
        }
    }
}

#[cfg(feature = "alignment")]
fn align_with_orb(
    images: &mut Vec<(PathBuf, DynamicImage)>,
) -> Result<Vec<DynamicImage>, HdrError> {
    use opencv::calib3d::{find_homography, RANSAC};
    use opencv::core::{BorderTypes, DMatch, Mat, Point2f, Scalar, Size, Vector, NORM_HAMMING};
    use opencv::features2d::{BFMatcher, ORB_ScoreType, ORB};
    use opencv::imgproc::{warp_perspective, INTER_LINEAR, WARP_INVERSE_MAP};
    use opencv::prelude::*;

    let ref_img = &images[0].1;
    let ref_gray = ref_img.to_luma8();
    let (ref_w, ref_h) = ref_gray.dimensions();

    let ref_mat = create_mat_from_gray(&ref_gray);
    let mut orb = ORB::create(2000, 1.2, 8, 31, 0, 2, ORB_ScoreType::HARRIS_SCORE, 31, 20)
        .map_err(|e| HdrError::Alignment(format!("ORB create: {}", e)))?;

    let mut ref_kps = Vector::new();
    let mut ref_desc = Mat::default();
    orb.detect_and_compute(
        &ref_mat,
        &mut Mat::default(),
        &mut ref_kps,
        &mut ref_desc,
        false,
    )
    .map_err(|e| HdrError::Alignment(format!("detect ref: {}", e)))?;

    if ref_desc.empty() {
        return Err(HdrError::Alignment("No features in reference".to_string()));
    }

    log::info!("Reference image: {} keypoints", ref_kps.len());

    let mut aligned_imgs: Vec<DynamicImage> = vec![images[0].1.clone()];

    for i in 1..images.len() {
        let curr_img = &images[i].1;
        let curr_gray = curr_img.to_luma8();
        let curr_mat = create_mat_from_gray(&curr_gray);

        let mut curr_kps = Vector::new();
        let mut curr_desc = Mat::default();
        orb.detect_and_compute(
            &curr_mat,
            &mut Mat::default(),
            &mut curr_kps,
            &mut curr_desc,
            false,
        )
        .map_err(|e| HdrError::Alignment(format!("detect curr: {}", e)))?;

        if curr_desc.empty() {
            log::warn!("No features in image {}", i);
            aligned_imgs.push(curr_img.clone());
            continue;
        }

        log::info!("Image {}: {} keypoints", i, curr_kps.len());

        let mut matcher = BFMatcher::create(NORM_HAMMING, false)
            .map_err(|e| HdrError::Alignment(format!("matcher: {}", e)))?;

        matcher
            .add(&curr_desc)
            .map_err(|e| HdrError::Alignment(format!("add: {}", e)))?;

        let mut matches = Vector::<DMatch>::new();
        matcher
            .match_(&ref_desc, &mut matches, &Mat::default())
            .map_err(|e| HdrError::Alignment(format!("match: {}", e)))?;

        log::info!("Image {}: {} matches", i, matches.len());

        if matches.len() < 15 {
            log::warn!("Not enough matches for image {}", i);
            aligned_imgs.push(curr_img.clone());
            continue;
        }

        let mut src = Vector::<Point2f>::new();
        let mut dst = Vector::<Point2f>::new();

        for m in matches.iter().take(50) {
            let q = m.query_idx as usize;
            let t = m.train_idx as usize;
            if q < ref_kps.len() && t < curr_kps.len() {
                src.push(ref_kps.get(q).unwrap().pt());
                dst.push(curr_kps.get(t).unwrap().pt());
            }
        }

        if src.len() < 10 {
            log::warn!("Not enough point correspondences for image {}", i);
            aligned_imgs.push(curr_img.clone());
            continue;
        }

        let homography = match find_homography(&src, &dst, &mut Mat::default(), RANSAC, 3.0) {
            Ok(h) => h,
            Err(e) => {
                log::warn!("Homography failed for image {}: {:?}", i, e);
                aligned_imgs.push(curr_img.clone());
                continue;
            }
        };

        let rgb = curr_img.to_rgb8();
        let curr_rgb_mat = create_mat_from_rgb8(&rgb);

        let mut warped = Mat::default();
        warp_perspective(
            &curr_rgb_mat,
            &mut warped,
            &homography,
            Size::new(ref_w as i32, ref_h as i32),
            INTER_LINEAR | WARP_INVERSE_MAP,
            i32::from(BorderTypes::BORDER_CONSTANT),
            Scalar::all(0.0),
        )
        .map_err(|e| HdrError::Alignment(format!("warp: {}", e)))?;

        let mut result = image::ImageBuffer::new(ref_w, ref_h);
        for y in 0..ref_h {
            for x in 0..ref_w {
                if let Ok(p) = warped.at_2d::<opencv::core::Vec3b>(y as i32, x as i32) {
                    result.put_pixel(x, y, image::Rgb([p[0], p[1], p[2]]));
                }
            }
        }

        aligned_imgs.push(DynamicImage::ImageRgb8(result));
    }

    Ok(aligned_imgs)
}

#[cfg(feature = "alignment")]
fn create_mat_from_gray(img: &image::GrayImage) -> opencv::core::Mat {
    use opencv::core::Mat;
    let (w, h) = img.dimensions();
    let w = w as usize;
    let h = h as usize;
    let rows: Vec<Vec<u8>> = (0..h)
        .map(|y| {
            (0..w)
                .map(|x| img.get_pixel(x as u32, y as u32)[0])
                .collect()
        })
        .collect();
    let rows_ref: Vec<&[u8]> = rows.iter().map(|r| r.as_slice()).collect();
    Mat::from_slice_2d(&rows_ref).unwrap()
}

#[cfg(feature = "alignment")]
fn create_mat_from_rgb8(img: &image::RgbImage) -> opencv::core::Mat {
    use opencv::core::Mat;
    let (w, h) = img.dimensions();
    let w = w as usize;
    let h = h as usize;
    let rows: Vec<Vec<u8>> = (0..h)
        .map(|y| {
            let mut row = Vec::with_capacity(w * 3);
            for x in 0..w {
                let p = img.get_pixel(x as u32, y as u32);
                row.push(p[0]);
                row.push(p[1]);
                row.push(p[2]);
            }
            row
        })
        .collect();
    let rows_ref: Vec<&[u8]> = rows.iter().map(|r| r.as_slice()).collect();
    Mat::from_slice_2d(&rows_ref).unwrap()
}

#[cfg(not(feature = "alignment"))]
pub fn align_images(_images: &mut Vec<(PathBuf, DynamicImage)>) -> Result<(), HdrError> {
    Err(HdrError::Alignment("Image alignment requires 'alignment' feature. Build with: cargo build --features alignment".to_string()))
}

pub fn warp_image(
    _image: &DynamicImage,
    _transform_matrix: &[f64; 9],
    _output_size: (u32, u32),
) -> Result<DynamicImage, HdrError> {
    Err(HdrError::Alignment(
        "Image warping not implemented".to_string(),
    ))
}
