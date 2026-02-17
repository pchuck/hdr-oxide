use image::DynamicImage;

pub struct Histogram {
    pub red: Vec<u32>,
    pub green: Vec<u32>,
    pub blue: Vec<u32>,
    pub luminance: Vec<u32>,
}

impl Histogram {
    pub fn new() -> Self {
        Self {
            red: vec![0; 256],
            green: vec![0; 256],
            blue: vec![0; 256],
            luminance: vec![0; 256],
        }
    }

    pub fn compute(image: &DynamicImage) -> Self {
        let mut hist = Self::new();
        let rgb_image = image.to_rgb8();

        for pixel in rgb_image.pixels() {
            let r = pixel[0] as usize;
            let g = pixel[1] as usize;
            let b = pixel[2] as usize;

            // Perceptual luminance
            let lum = ((0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) as u8) as usize;

            hist.red[r] += 1;
            hist.green[g] += 1;
            hist.blue[b] += 1;
            hist.luminance[lum] += 1;
        }

        hist
    }

    pub fn max_count(&self) -> u32 {
        let max_red = self.red.iter().max().copied().unwrap_or(0);
        let max_green = self.green.iter().max().copied().unwrap_or(0);
        let max_blue = self.blue.iter().max().copied().unwrap_or(0);
        let max_lum = self.luminance.iter().max().copied().unwrap_or(0);

        max_red.max(max_green).max(max_blue).max(max_lum)
    }

    pub fn clear(&mut self) {
        for i in 0..256 {
            self.red[i] = 0;
            self.green[i] = 0;
            self.blue[i] = 0;
            self.luminance[i] = 0;
        }
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::RgbImage;

    #[test]
    fn test_histogram_new() {
        let hist = Histogram::new();
        assert_eq!(hist.red.len(), 256);
        assert_eq!(hist.green.len(), 256);
        assert_eq!(hist.blue.len(), 256);
        assert_eq!(hist.luminance.len(), 256);
        assert!(hist.red.iter().all(|&v| v == 0));
    }

    #[test]
    fn test_histogram_compute() {
        let mut img = RgbImage::new(10, 10);
        for y in 0..10 {
            for x in 0..10 {
                img.put_pixel(x, y, image::Rgb([128, 64, 192]));
            }
        }
        let dynamic_img = DynamicImage::ImageRgb8(img);

        let hist = Histogram::compute(&dynamic_img);

        assert_eq!(hist.red[128], 100);
        assert_eq!(hist.green[64], 100);
        assert_eq!(hist.blue[192], 100);
        assert!(hist.luminance.iter().sum::<u32>() == 100);
    }
}
