use crate::error::HdrError;
use crate::image::histogram::Histogram;
use crate::image::merge::HdrImage;
use crate::image::tonemap::{tonemap_hdr_arc_with_progress, TonemapSettings};
use eframe::egui;
use image::{DynamicImage, GenericImageView};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

const PREVIEW_SIZE: u32 = 256;
const WORKING_MAX_DIMENSION: u32 = 2048;

pub struct SourceImages {
    full: Arc<DynamicImage>,
    working: Arc<DynamicImage>,
    exposure_seconds: f32,
}

impl SourceImages {
    fn new(img: DynamicImage, exposure_seconds: f32) -> Self {
        let working = Self::resize_to_working(&img);
        Self {
            full: Arc::new(img),
            working: Arc::new(working),
            exposure_seconds,
        }
    }

    fn resize_to_working(img: &DynamicImage) -> DynamicImage {
        let (w, h) = img.dimensions();
        let max_dim = w.max(h);
        if max_dim <= WORKING_MAX_DIMENSION {
            return img.clone();
        }
        let scale = WORKING_MAX_DIMENSION as f32 / max_dim as f32;
        let new_w = (w as f32 * scale) as u32;
        let new_h = (h as f32 * scale) as u32;
        img.resize(new_w, new_h, image::imageops::FilterType::Lanczos3)
    }
}

pub enum GuiCommand {
    MergeComplete(Result<HdrImage, String>),
    LoadError(String),
    SaveComplete(Result<(), String>),
    PreviewComplete(Result<image::RgbaImage, String>),
    ImageLoaded(PathBuf, SourceImages, egui::ColorImage),
    FilesSelected(Vec<PathBuf>),
    Progress {
        stage: String,
        current: usize,
        total: usize,
    },
    CompareTextureReady(egui::ColorImage),
}

pub struct HdrApp {
    input_paths: Vec<PathBuf>,
    preloaded_images: Vec<(PathBuf, SourceImages)>,
    preloaded_textures: std::collections::HashMap<PathBuf, egui::TextureHandle>,
    loading_paths: Vec<PathBuf>,
    pending_load_queue: Vec<PathBuf>,
    is_loading: bool,
    total_loading: usize,
    loaded_count: usize,
    needs_hdr: bool,
    settings_changed: bool,
    last_tonemap_method: String,
    last_exposure: f32,
    last_contrast: f32,
    last_saturation: f32,
    last_vibrance: f32,
    last_shadows: f32,
    last_highlights: f32,
    last_temperature: f32,
    last_tint: f32,
    last_hue_shift: f32,
    last_sharpen: f32,
    hdr_image: Option<Arc<HdrImage>>,
    preview_texture: Option<egui::TextureHandle>,
    tonemap_method: String,
    exposure: f32,
    contrast: f32,
    saturation: f32,
    vibrance: f32,
    shadows: f32,
    highlights: f32,
    temperature: f32,
    tint: f32,
    hue_shift: f32,
    sharpen: f32,
    status: String,
    is_generating: bool,
    progress_stage: String,
    progress_current: usize,
    progress_total: usize,
    rx: Option<mpsc::Receiver<GuiCommand>>,
    path_input: String,
    show_about: bool,
    about_texture: Option<egui::TextureHandle>,
    cancel_token: Option<Arc<AtomicBool>>,
    compare_mode: bool,
    compare_position: f32,
    compare_index: usize,
    compare_texture: Option<egui::TextureHandle>,
    histogram: Histogram,
    show_histogram: bool,
}

impl Default for HdrApp {
    fn default() -> Self {
        Self {
            input_paths: Vec::new(),
            preloaded_images: Vec::new(),
            preloaded_textures: std::collections::HashMap::new(),
            loading_paths: Vec::new(),
            pending_load_queue: Vec::new(),
            is_loading: false,
            total_loading: 0,
            loaded_count: 0,
            needs_hdr: false,
            settings_changed: false,
            last_tonemap_method: "reinhard".to_string(),
            last_exposure: 1.0,
            last_contrast: 1.0,
            last_saturation: 1.0,
            last_vibrance: 0.0,
            last_shadows: 0.0,
            last_highlights: 0.0,
            last_temperature: 0.0,
            last_tint: 0.0,
            last_hue_shift: 0.0,
            last_sharpen: 0.0,
            hdr_image: None,
            preview_texture: None,
            tonemap_method: "reinhard".to_string(),
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
            status: "Add images using 'Open Files' button".to_string(),
            is_generating: false,
            progress_stage: String::new(),
            progress_current: 0,
            progress_total: 0,
            rx: None,
            path_input: String::new(),
            show_about: false,
            about_texture: None,
            cancel_token: None,
            compare_mode: false,
            compare_position: 0.5,
            compare_index: 0,
            compare_texture: None,
            histogram: Histogram::new(),
            show_histogram: false,
        }
    }
}

impl HdrApp {
    fn add_path(&mut self) {
        let input = &self.path_input;

        if input.contains('*') || input.contains('?') {
            match glob::glob(input) {
                Ok(paths) => {
                    let valid_exts = ["jpg", "jpeg", "png", "tif", "tiff"];
                    let mut paths_to_load = Vec::new();

                    for path in paths.flatten() {
                        if path.is_file() {
                            let ext = path
                                .extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("")
                                .to_lowercase();
                            if valid_exts.contains(&ext.as_str())
                                && !self.input_paths.contains(&path)
                                && !self.loading_paths.contains(&path)
                            {
                                paths_to_load.push(path.clone());
                            }
                        }
                    }

                    if !paths_to_load.is_empty() {
                        self.input_paths.extend(paths_to_load.clone());
                        self.needs_hdr = true;
                        self.settings_changed = true;
                        self.status = format!(
                            "{} images (loading {} from pattern)",
                            self.input_paths.len(),
                            paths_to_load.len()
                        );
                        self.start_async_loading(paths_to_load);
                    } else {
                        self.status = "No matching image files found".to_string();
                    }
                }
                Err(e) => {
                    self.status = format!("Invalid pattern: {}", e);
                }
            }
        } else {
            let path = PathBuf::from(input);
            if path.exists() && path.is_file() {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if ["jpg", "jpeg", "png", "tif", "tiff"].contains(&ext.as_str()) {
                    if !self.input_paths.contains(&path) && !self.loading_paths.contains(&path) {
                        self.input_paths.push(path.clone());
                        self.needs_hdr = true;
                        self.settings_changed = true;
                        self.status = format!("{} images (loading)", self.input_paths.len());
                        self.start_async_loading(vec![path]);
                    }
                } else {
                    self.status = "Not an image file".to_string();
                }
            } else {
                self.status = "File not found".to_string();
            }
        }
        self.path_input.clear();
    }

    fn add_multiple_paths(&mut self, paths: &[PathBuf]) {
        let valid_exts = ["jpg", "jpeg", "png", "tif", "tiff"];
        let mut paths_to_load = Vec::new();

        for path in paths {
            if path.exists() && path.is_file() {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if valid_exts.contains(&ext.as_str())
                    && !self.input_paths.contains(path)
                    && !self.loading_paths.contains(path)
                {
                    paths_to_load.push(path.clone());
                }
            }
        }

        if !paths_to_load.is_empty() {
            self.input_paths.extend(paths_to_load.clone());
            self.needs_hdr = true;
            self.status = format!("{} images (loading)", self.input_paths.len());
            self.start_async_loading(paths_to_load);
        } else {
            self.status = "No valid image files selected".to_string();
        }
    }

    fn start_async_loading(&mut self, paths: Vec<PathBuf>) {
        if self.is_loading {
            self.pending_load_queue.extend(paths);
            return;
        }

        self.is_loading = true;
        self.total_loading = paths.len();
        self.loaded_count = 0;
        self.loading_paths = paths.clone();
        self.status = format!("Loading 0/{} images...", self.total_loading);

        let cancel_token = Arc::new(AtomicBool::new(false));
        self.cancel_token = Some(Arc::clone(&cancel_token));

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        thread::spawn(move || {
            use rayon::prelude::*;

            paths.into_par_iter().for_each(|path| {
                if cancel_token.load(Ordering::Relaxed) {
                    return;
                }
                let result = image::open(&path);
                if cancel_token.load(Ordering::Relaxed) {
                    return;
                }
                match result {
                    Ok(img) => {
                        let exposure = crate::image::loader::extract_exposure_time(&path)
                            .unwrap_or_else(|_| {
                                log::warn!(
                                    "Could not read exposure from {:?}, assuming 1/125",
                                    path
                                );
                                1.0 / 125.0
                            });

                        let source_images = SourceImages::new(img, exposure);

                        let resized = source_images.working.resize(
                            PREVIEW_SIZE,
                            PREVIEW_SIZE,
                            image::imageops::FilterType::Lanczos3,
                        );
                        let rgba = resized.to_rgba8();
                        let (width, height) = rgba.dimensions();
                        let pixels: Vec<egui::Color32> = rgba
                            .pixels()
                            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                            .collect();
                        let color_image = egui::ColorImage {
                            size: [width as usize, height as usize],
                            pixels,
                        };
                        let _ = tx.send(GuiCommand::ImageLoaded(path, source_images, color_image));
                    }
                    Err(e) => {
                        let _ = tx.send(GuiCommand::LoadError(format!("Failed to load: {}", e)));
                    }
                }
            });
        });
    }

    fn process_queued_loads(&mut self) {
        if !self.pending_load_queue.is_empty() && !self.is_loading {
            let paths = std::mem::take(&mut self.pending_load_queue);
            self.start_async_loading(paths);
        }
    }

    fn is_loading(&self) -> bool {
        !self.loading_paths.is_empty()
    }

    fn check_settings_changed(&mut self) {
        self.settings_changed = self.tonemap_method != self.last_tonemap_method
            || (self.exposure - self.last_exposure).abs() > 0.001
            || (self.contrast - self.last_contrast).abs() > 0.001
            || (self.saturation - self.last_saturation).abs() > 0.001
            || (self.vibrance - self.last_vibrance).abs() > 0.1
            || (self.shadows - self.last_shadows).abs() > 0.1
            || (self.highlights - self.last_highlights).abs() > 0.1
            || (self.temperature - self.last_temperature).abs() > 0.1
            || (self.tint - self.last_tint).abs() > 0.1
            || (self.hue_shift - self.last_hue_shift).abs() > 0.1
            || (self.sharpen - self.last_sharpen).abs() > 0.1;
    }

    fn save_last_settings(&mut self) {
        self.last_tonemap_method = self.tonemap_method.clone();
        self.last_exposure = self.exposure;
        self.last_contrast = self.contrast;
        self.last_saturation = self.saturation;
        self.last_vibrance = self.vibrance;
        self.last_shadows = self.shadows;
        self.last_highlights = self.highlights;
        self.last_temperature = self.temperature;
        self.last_tint = self.tint;
        self.last_hue_shift = self.hue_shift;
        self.last_sharpen = self.sharpen;
        self.settings_changed = false;
    }

    fn clear_images(&mut self) {
        if let Some(token) = &self.cancel_token {
            token.store(true, Ordering::Relaxed);
        }
        self.cancel_token = None;
        self.input_paths.clear();
        self.preloaded_images.clear();
        self.preloaded_textures.clear();
        self.loading_paths.clear();
        self.pending_load_queue.clear();
        self.is_loading = false;
        self.is_generating = false;
        self.total_loading = 0;
        self.loaded_count = 0;
        self.hdr_image = None;
        self.preview_texture = None;
        self.compare_mode = false;
        self.compare_texture = None;
        self.status = "Cleared".to_string();
    }

    fn open_file_dialog(&mut self) {
        if self.is_generating || self.is_loading {
            return;
        }

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        self.is_loading = true;
        self.status = "Opening file dialog...".to_string();

        thread::spawn(move || {
            let files = rfd::FileDialog::new()
                .add_filter("Images", &["jpg", "jpeg", "png", "tif", "tiff"])
                .set_title("Select Images for HDR")
                .pick_files();

            let _ = tx.send(GuiCommand::FilesSelected(files.unwrap_or_default()));
        });
    }

    fn start_merge(&mut self) {
        if self.input_paths.is_empty() {
            self.status = "No images".to_string();
            return;
        }

        if self.preloaded_images.is_empty() {
            self.status = "Images not loaded".to_string();
            return;
        }

        let preloaded: Vec<(PathBuf, Arc<DynamicImage>, f32)> = self
            .preloaded_images
            .iter()
            .map(|(path, src)| (path.clone(), Arc::clone(&src.working), src.exposure_seconds))
            .collect();

        self.is_generating = true;
        self.progress_stage = "Merging HDR".to_string();
        self.progress_current = 0;
        self.progress_total = 100;
        self.status = "Merging HDR...".to_string();

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        thread::spawn(move || {
            use crate::image::loader::SourceImage;

            let mut images = Vec::with_capacity(preloaded.len());

            for (path, img, exposure) in preloaded {
                images.push(SourceImage::new(path, (*img).clone(), exposure));
            }

            let total_pixels = images[0].width as usize * images[0].height as usize;
            let tx_progress = tx.clone();
            let result =
                crate::image::merge::merge_to_hdr_parallel_with_progress(&images, move |count| {
                    let _ = tx_progress.send(GuiCommand::Progress {
                        stage: "Merging HDR".to_string(),
                        current: count,
                        total: total_pixels,
                    });
                });

            match result {
                Ok(hdr) => {
                    let _ = tx.send(GuiCommand::MergeComplete(Ok(hdr)));
                }
                Err(e) => {
                    let _ = tx.send(GuiCommand::MergeComplete(Err(format!("HDR failed: {}", e))));
                }
            }
        });
    }

    fn check_results(&mut self, ctx: &egui::Context) {
        if self.rx.is_none() {
            return;
        }

        let mut commands = Vec::new();
        if let Some(rx) = &self.rx {
            while let Ok(cmd) = rx.try_recv() {
                commands.push(cmd);
            }
        }

        for cmd in commands {
            match cmd {
                GuiCommand::MergeComplete(Ok(hdr)) => {
                    self.hdr_image = Some(Arc::new(hdr));
                    self.status = "HDR ready".to_string();
                    self.is_generating = false;
                    self.needs_hdr = false;
                    self.rx = None;
                    self.update_preview(ctx);
                }
                GuiCommand::MergeComplete(Err(e)) => {
                    self.status = e;
                    self.is_generating = false;
                    self.rx = None;
                }
                GuiCommand::LoadError(e) => {
                    self.status = e;
                    self.is_generating = false;
                    self.is_loading = false;
                    self.rx = None;
                }
                GuiCommand::SaveComplete(result) => {
                    match result {
                        Ok(()) => self.status = "Saved successfully".to_string(),
                        Err(e) => self.status = format!("Save failed: {}", e),
                    }
                    self.is_generating = false;
                    self.rx = None;
                }
                GuiCommand::PreviewComplete(result) => {
                    self.is_generating = false;
                    self.rx = None;
                    match result {
                        Ok(image) => {
                            self.histogram =
                                Histogram::compute(&DynamicImage::ImageRgba8(image.clone()));
                            self.apply_preview_texture(ctx, image);
                            self.save_last_settings();
                            if self.compare_mode {
                                self.compare_texture = None;
                                self.start_compare_texture_load();
                            }
                        }
                        Err(e) => {
                            self.status = format!("Tonemap failed: {}", e);
                        }
                    }
                }
                GuiCommand::ImageLoaded(path, img, thumbnail) => {
                    self.loading_paths.retain(|p| *p != path);
                    self.preloaded_images.push((path.clone(), img));
                    self.loaded_count += 1;
                    self.status = format!(
                        "Loading {}/{} images...",
                        self.loaded_count, self.total_loading
                    );

                    let texture = ctx.load_texture(
                        format!(
                            "img_{}",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        Arc::new(thumbnail),
                        egui::TextureOptions::LINEAR,
                    );

                    self.preloaded_textures.insert(path.clone(), texture);

                    if self.loading_paths.is_empty() {
                        self.status = format!("{} images ready", self.input_paths.len());
                        self.is_generating = false;
                        self.is_loading = false;
                        self.rx = None;
                        self.process_queued_loads();
                    }
                }
                GuiCommand::FilesSelected(paths) => {
                    self.is_loading = false;
                    if paths.is_empty() {
                        self.status = "No files selected".to_string();
                        self.rx = None;
                    } else {
                        self.add_multiple_paths(&paths);
                    }
                }
                GuiCommand::Progress {
                    stage,
                    current,
                    total,
                } => {
                    self.progress_stage = stage;
                    self.progress_current = current;
                    self.progress_total = total;
                }
                GuiCommand::CompareTextureReady(color_image) => {
                    self.update_compare_texture(ctx, color_image);
                    self.rx = None;
                }
            }
        }
    }

    fn update_preview(&mut self, _ctx: &egui::Context) {
        if self.hdr_image.is_none() || self.is_generating {
            return;
        }

        self.is_generating = true;
        self.progress_stage = "Tonemapping".to_string();
        self.progress_current = 0;
        self.progress_total = 100;
        self.status = "Tonemapping...".to_string();

        let hdr_arc = Arc::clone(self.hdr_image.as_ref().unwrap());
        let tonemap_method = self.tonemap_method.clone();
        let settings = TonemapSettings {
            exposure: self.exposure,
            contrast: self.contrast,
            saturation: self.saturation,
            vibrance: self.vibrance / 100.0,
            shadows: self.shadows / 100.0,
            highlights: self.highlights / 100.0,
            temperature: self.temperature,
            tint: self.tint,
            hue_shift: self.hue_shift / 360.0,
            sharpen: self.sharpen / 100.0,
        };

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        thread::spawn(move || -> () {
            let total_pixels = {
                let hdr = hdr_arc.as_ref();
                (hdr.width as u64 * hdr.height as u64) as usize
            };

            let tx_progress = tx.clone();
            let result = crate::image::tonemap::tonemap_hdr_arc_with_progress(
                hdr_arc,
                &tonemap_method,
                &settings,
                move |count| {
                    let _ = tx_progress.send(GuiCommand::Progress {
                        stage: "Tonemapping".to_string(),
                        current: count,
                        total: total_pixels,
                    });
                },
            );

            match result {
                Ok(tonemapped) => {
                    let rgba = tonemapped.to_rgba8();
                    let _ = tx.send(GuiCommand::PreviewComplete(Ok(rgba)));
                }
                Err(e) => {
                    let _ = tx.send(GuiCommand::PreviewComplete(Err(e.to_string())));
                }
            }
        });
    }

    fn apply_preview_texture(&mut self, ctx: &egui::Context, image: image::RgbaImage) {
        let (width, height) = image.dimensions();
        let pixels: Vec<egui::Color32> = image
            .pixels()
            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
            .collect();

        let color_image = egui::ColorImage {
            size: [width as usize, height as usize],
            pixels,
        };

        self.preview_texture = Some(ctx.load_texture(
            "preview",
            Arc::new(color_image),
            egui::TextureOptions::LINEAR,
        ));
        self.status = "Preview ready".to_string();
    }

    fn toggle_compare_mode(&mut self, _ctx: &egui::Context) {
        self.compare_mode = !self.compare_mode;
        if self.compare_mode {
            self.compare_position = 0.5;
            self.compare_index = 0;
            self.compare_texture = None;
            self.start_compare_texture_load();
        } else {
            self.compare_texture = None;
        }
    }

    fn start_compare_texture_load(&mut self) {
        if self.compare_index >= self.preloaded_images.len() {
            return;
        }

        if self.rx.is_some() {
            return;
        }

        let img = Arc::clone(&self.preloaded_images[self.compare_index].1.working);
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        thread::spawn(move || {
            let size = img.dimensions();
            let rgba = img.to_rgba8();
            let pixels: Vec<egui::Color32> = rgba
                .pixels()
                .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                .collect();
            let color_image = egui::ColorImage {
                size: [size.0 as usize, size.1 as usize],
                pixels,
            };
            let _ = tx.send(GuiCommand::CompareTextureReady(color_image));
        });
    }

    fn update_compare_texture(&mut self, ctx: &egui::Context, color_image: egui::ColorImage) {
        self.compare_texture = Some(ctx.load_texture(
            "compare_source",
            Arc::new(color_image),
            egui::TextureOptions::LINEAR,
        ));
    }

    fn save_output(&mut self) {
        if self.preloaded_images.is_empty() || self.is_generating {
            return;
        }

        self.is_generating = true;
        self.progress_stage = "Exporting".to_string();
        self.progress_current = 0;
        self.progress_total = 100;
        self.status = "Preparing full resolution HDR...".to_string();

        let preloaded: Vec<(PathBuf, Arc<DynamicImage>, f32)> = self
            .preloaded_images
            .iter()
            .map(|(path, src)| (path.clone(), Arc::clone(&src.full), src.exposure_seconds))
            .collect();

        let tonemap_method = self.tonemap_method.clone();
        let settings = TonemapSettings {
            exposure: self.exposure,
            contrast: self.contrast,
            saturation: self.saturation,
            vibrance: self.vibrance / 100.0,
            shadows: self.shadows / 100.0,
            highlights: self.highlights / 100.0,
            temperature: self.temperature,
            tint: self.tint,
            hue_shift: self.hue_shift / 360.0,
            sharpen: self.sharpen / 100.0,
        };

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        thread::spawn(move || -> () {
            use crate::image::loader::SourceImage;

            let mut images = Vec::with_capacity(preloaded.len());
            for (path, img, exposure) in preloaded {
                images.push(SourceImage::new(path, (*img).clone(), exposure));
            }

            let tx_progress = tx.clone();
            let hdr_result =
                crate::image::merge::merge_to_hdr_parallel_with_progress(&images, |count| {
                    let _ = tx_progress.send(GuiCommand::Progress {
                        stage: "Merging full-res HDR".to_string(),
                        current: count,
                        total: images[0].width as usize * images[0].height as usize,
                    });
                });

            let hdr = match hdr_result {
                Ok(h) => h,
                Err(e) => {
                    let _ = tx.send(GuiCommand::SaveComplete(Err(format!(
                        "HDR merge failed: {}",
                        e
                    ))));
                    return;
                }
            };

            let _ = tx.send(GuiCommand::Progress {
                stage: "Tonemapping".to_string(),
                current: 0,
                total: 100,
            });

            let total_pixels = (hdr.width as u64 * hdr.height as u64) as usize;
            let hdr_arc = Arc::new(hdr);
            let tx_progress2 = tx.clone();
            let tonemapped = match tonemap_hdr_arc_with_progress(
                hdr_arc,
                &tonemap_method,
                &settings,
                move |count| {
                    let _ = tx_progress2.send(GuiCommand::Progress {
                        stage: "Tonemapping".to_string(),
                        current: count,
                        total: total_pixels,
                    });
                },
            ) {
                Ok(t) => t,
                Err(e) => {
                    let _ = tx.send(GuiCommand::SaveComplete(Err(format!(
                        "Tonemap failed: {}",
                        e
                    ))));
                    return;
                }
            };

            let _ = tx.send(GuiCommand::Progress {
                stage: "Saving".to_string(),
                current: 1,
                total: 1,
            });

            let file = rfd::FileDialog::new()
                .add_filter("PNG", &["png"])
                .add_filter("JPEG", &["jpg", "jpeg"])
                .set_title("Save HDR")
                .set_file_name("hdr_output.png")
                .save_file();

            match file {
                Some(path) => match tonemapped.save(&path) {
                    Ok(()) => {
                        let _ = tx.send(GuiCommand::SaveComplete(Ok(())));
                    }
                    Err(e) => {
                        let _ = tx.send(GuiCommand::SaveComplete(Err(e.to_string())));
                    }
                },
                None => {
                    let _ = tx.send(GuiCommand::SaveComplete(
                        Err("No file selected".to_string()),
                    ));
                }
            }
        });
    }

    fn reset_adjustments(&mut self) {
        self.exposure = 1.0;
        self.contrast = 1.0;
        self.saturation = 1.0;
        self.vibrance = 0.0;
        self.shadows = 0.0;
        self.highlights = 0.0;
        self.temperature = 0.0;
        self.tint = 0.0;
        self.hue_shift = 0.0;
        self.sharpen = 0.0;
        self.status = "Adjustments reset".to_string();
    }
}

impl eframe::App for HdrApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_results(ctx);

        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            ui.heading("HDR-Oxide");

            ui.separator();

            ui.collapsing("Source Images", |ui| {
                ui.label("Add images:");
                if ui.button("Open Files...").clicked() && !self.is_generating && !self.is_loading {
                    self.open_file_dialog();
                }

                ui.separator();

                ui.label("Or type a path (supports wildcards):");
                ui.text_edit_singleline(&mut self.path_input);
                if ui.button("Add File(s)").clicked() {
                    self.add_path();
                }

                if !self.input_paths.is_empty() {
                    ui.separator();
                    ui.label(format!("{} images:", self.input_paths.len()));

                    let panel_width = ui.available_width();
                    let to_remove = egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            let mut remove_idx = None;
                            let spacing = 4.0;

                            for (i, path) in self.input_paths.iter().enumerate() {
                                ui.vertical(|ui| {
                                    ui.set_width(panel_width);

                                    let texture_found = self.preloaded_textures.get(path);

                                    if let Some(texture) = texture_found {
                                        let size = texture.size();
                                        let aspect = size[1] as f32 / size[0] as f32;
                                        let height = (panel_width - spacing * 2.0) * aspect;
                                        let max_height = 120.0f32;
                                        let final_height = height.min(max_height);

                                        ui.add(egui::Image::new(texture).fit_to_exact_size(
                                            egui::vec2(panel_width - spacing * 2.0, final_height),
                                        ));
                                    } else {
                                        ui.add_sized(
                                            egui::vec2(panel_width - spacing * 2.0, 80.0),
                                            egui::Spinner::new().size(24.0),
                                        );
                                    }

                                    ui.horizontal(|ui| {
                                        ui.set_width(panel_width - spacing * 2.0);
                                        let filename = path
                                            .file_name()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .chars()
                                            .take(25)
                                            .collect::<String>();
                                        ui.label(
                                            egui::RichText::new(filename).size(11.0).monospace(),
                                        );
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if ui.button("×").clicked() {
                                                    remove_idx = Some(i);
                                                }
                                            },
                                        );
                                    });

                                    ui.add_space(spacing);
                                });
                            }
                            remove_idx
                        });

                    if let Some(idx) = to_remove.inner {
                        let path = self.input_paths.remove(idx);
                        self.preloaded_images.retain(|(p, _)| *p != path);
                        self.preloaded_textures.remove(&path);
                        self.loading_paths.retain(|p| *p != path);
                        self.hdr_image = None;
                        self.preview_texture = None;
                        self.compare_mode = false;
                        self.compare_texture = None;
                        self.needs_hdr = true;
                        self.settings_changed = true;
                    }

                    if ui.button("Clear All").clicked() {
                        self.clear_images();
                    }
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                let can_create_hdr = self.needs_hdr
                    && !self.input_paths.is_empty()
                    && !self.is_generating
                    && !self.is_loading();

                if ui
                    .add_enabled(can_create_hdr, egui::Button::new("Generate HDR"))
                    .clicked()
                {
                    self.start_merge();
                }

                let can_save =
                    self.hdr_image.is_some() && !self.settings_changed && !self.is_generating;
                if ui
                    .add_enabled(can_save, egui::Button::new("Save HDR"))
                    .clicked()
                {
                    self.save_output();
                }

                if self.is_loading() {
                    ui.label(egui::RichText::new("Loading images...").small());
                }

                if self.is_generating {
                    ui.label(egui::RichText::new("Generating HDR...").small());
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                let can_apply =
                    self.hdr_image.is_some() && self.settings_changed && !self.is_generating;
                if ui
                    .add_enabled(can_apply, egui::Button::new("Apply"))
                    .clicked()
                {
                    self.update_preview(ctx);
                }

                let can_compare =
                    self.preview_texture.is_some() && !self.preloaded_images.is_empty();
                if ui
                    .add_enabled(can_compare, egui::Button::new("Compare"))
                    .clicked()
                {
                    self.toggle_compare_mode(ctx);
                }

                let can_histogram = self.preview_texture.is_some();
                if ui
                    .add_enabled(can_histogram, egui::Button::new("Histogram"))
                    .clicked()
                {
                    self.show_histogram = !self.show_histogram;
                }
            });

            if self.compare_mode && !self.preloaded_images.is_empty() {
                ui.horizontal(|ui| {
                    ui.label("Compare with:");
                    let num_images = self.preloaded_images.len();
                    if ui.button("◀").clicked() {
                        self.compare_index = if self.compare_index == 0 {
                            num_images - 1
                        } else {
                            self.compare_index - 1
                        };
                        self.compare_texture = None;
                        self.start_compare_texture_load();
                    }
                    ui.label(format!("{}/{}", self.compare_index + 1, num_images));
                    if ui.button("▶").clicked() {
                        self.compare_index = (self.compare_index + 1) % num_images;
                        self.compare_texture = None;
                        self.start_compare_texture_load();
                    }
                    if self.compare_texture.is_none() {
                        ui.add(egui::Spinner::new().size(16.0));
                    }
                });
            }

            ui.label("Tonemap:");
            egui::ComboBox::from_label("Method")
                .selected_text(&self.tonemap_method)
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.tonemap_method,
                        "reinhard".to_string(),
                        "Reinhard",
                    );
                    ui.selectable_value(&mut self.tonemap_method, "filmic".to_string(), "Filmic");
                    ui.selectable_value(&mut self.tonemap_method, "gamma".to_string(), "Gamma");
                });

            self.check_settings_changed();

            ui.add(egui::Slider::new(&mut self.exposure, 0.1..=10.0).text("Exposure"));
            ui.add(egui::Slider::new(&mut self.contrast, 0.0..=2.0).text("Contrast"));

            self.check_settings_changed();

            ui.collapsing("Advanced Controls", |ui| {
                ui.add(egui::Slider::new(&mut self.saturation, 0.0..=2.0).text("Saturation"));
                ui.add(egui::Slider::new(&mut self.vibrance, -100.0..=100.0).text("Vibrance"));
                ui.add(egui::Slider::new(&mut self.shadows, -100.0..=100.0).text("Shadows"));
                ui.add(egui::Slider::new(&mut self.highlights, -100.0..=100.0).text("Highlights"));
                ui.add(
                    egui::Slider::new(&mut self.temperature, -100.0..=100.0).text("Temperature"),
                );
                ui.add(egui::Slider::new(&mut self.tint, -100.0..=100.0).text("Tint"));
                ui.add(egui::Slider::new(&mut self.hue_shift, -180.0..=180.0).text("Hue Shift"));
                ui.add(egui::Slider::new(&mut self.sharpen, -100.0..=100.0).text("Sharpen/Blur"));

                if ui.button("Reset All").clicked() {
                    self.reset_adjustments();
                }
            });

            ui.separator();

            if ui.button("About").clicked() {
                self.show_about = true;
                if self.about_texture.is_none() {
                    if let Some(img) = load_about_image() {
                        let (width, height) = img.dimensions();
                        let rgba = img.to_rgba8();
                        let pixels: Vec<egui::Color32> = rgba
                            .pixels()
                            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                            .collect();
                        let color_image = egui::ColorImage {
                            size: [width as usize, height as usize],
                            pixels,
                        };
                        self.about_texture = Some(ctx.load_texture(
                            "about_image",
                            Arc::new(color_image),
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                }
            }

            ui.separator();

            if self.is_loading && self.total_loading > 0 {
                let progress = self.loaded_count as f32 / self.total_loading as f32;
                let progress_bar = egui::ProgressBar::new(progress).text(format!(
                    "Loading {}/{}",
                    self.loaded_count, self.total_loading
                ));
                ui.add(progress_bar);
            } else if self.is_generating && self.progress_total > 0 {
                let progress = self.progress_current as f32 / self.progress_total as f32;
                let progress_bar = egui::ProgressBar::new(progress).text(format!(
                    "{}: {:.0}%",
                    self.progress_stage,
                    progress * 100.0
                ));
                ui.add(progress_bar);
            } else if self.is_generating {
                ui.add(egui::Spinner::new());
            }
            ui.label(&self.status);
        });

        if self.show_about {
            egui::Window::new("About HDR Oxide")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        if let Some(ref texture) = self.about_texture {
                            let size = texture.size();
                            let max_width = 400.0;
                            let scale = (max_width / size[0] as f32).min(1.0);
                            let display_size =
                                egui::vec2(size[0] as f32 * scale, size[1] as f32 * scale);
                            ui.add(egui::Image::new(texture).max_size(display_size));
                            ui.add_space(10.0);
                        }

                        ui.heading("HDR Oxide");
                        ui.label("Copyright © 2026 ultrametrics");
                        ui.add_space(10.0);
                        ui.label("CLI: hdr-oxide create --help");
                        ui.add_space(10.0);

                        if ui.button("Close").clicked() {
                            self.show_about = false;
                        }
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.compare_mode {
                if let (Some(preview), Some(compare)) =
                    (&self.preview_texture, &self.compare_texture)
                {
                    let preview_size = preview.size();
                    let available = ui.available_size();
                    let scale = (available.x / preview_size[0] as f32)
                        .min(available.y / preview_size[1] as f32)
                        .min(1.0);
                    let display_size = egui::vec2(
                        preview_size[0] as f32 * scale,
                        preview_size[1] as f32 * scale,
                    );

                    let (rect, response) =
                        ui.allocate_exact_size(display_size, egui::Sense::drag());

                    let divider_x = rect.left() + rect.width() * self.compare_position;

                    let mut clip_rect_left = ui.clip_rect();
                    clip_rect_left.max.x = divider_x;

                    let mut clip_rect_right = ui.clip_rect();
                    clip_rect_right.min.x = divider_x;

                    {
                        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                        child_ui.set_clip_rect(clip_rect_left);
                        child_ui.image((compare.id(), display_size));
                    }

                    {
                        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                        child_ui.set_clip_rect(clip_rect_right);
                        child_ui.image((preview.id(), display_size));
                    }

                    ui.painter().line_segment(
                        [
                            egui::pos2(divider_x, rect.top()),
                            egui::pos2(divider_x, rect.bottom()),
                        ],
                        egui::Stroke::new(2.0, egui::Color32::WHITE),
                    );

                    let handle_radius = 8.0;
                    let handle_center = egui::pos2(divider_x, rect.center().y);
                    ui.painter()
                        .circle_filled(handle_center, handle_radius, egui::Color32::WHITE);
                    ui.painter().circle_stroke(
                        handle_center,
                        handle_radius,
                        egui::Stroke::new(2.0, egui::Color32::BLACK),
                    );

                    let label_pos = egui::pos2(rect.left() + 10.0, rect.top() + 10.0);
                    ui.painter().text(
                        label_pos,
                        egui::Align2::LEFT_TOP,
                        "Original",
                        egui::FontId::default(),
                        egui::Color32::from_rgba_premultiplied(255, 255, 255, 200),
                    );
                    let label_pos = egui::pos2(rect.right() - 10.0, rect.top() + 10.0);
                    ui.painter().text(
                        label_pos,
                        egui::Align2::RIGHT_TOP,
                        "HDR",
                        egui::FontId::default(),
                        egui::Color32::from_rgba_premultiplied(255, 255, 255, 200),
                    );

                    if response.dragged() {
                        if let Some(pos) = response.interact_pointer_pos() {
                            let new_pos = (pos.x - rect.left()) / rect.width();
                            self.compare_position = new_pos.clamp(0.0, 1.0);
                        }
                    }

                    ui.label("Drag to compare");
                } else {
                    ui.label("Loading comparison...");
                }
            } else if let Some(ref texture) = self.preview_texture {
                ui.heading("Preview");

                let size = texture.size();
                let available = ui.available_size();
                let scale = (available.x / size[0] as f32)
                    .min(available.y / size[1] as f32)
                    .min(1.0);
                let scaled_size = egui::vec2(size[0] as f32 * scale, size[1] as f32 * scale);

                let (rect, _response) = ui.allocate_exact_size(scaled_size, egui::Sense::hover());
                ui.painter().image(
                    texture.id(),
                    rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );

                // Draw histogram overlay
                if self.show_histogram {
                    let hist_height = 100.0;
                    let hist_width = rect.width();
                    let hist_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.left(), rect.bottom() - hist_height),
                        egui::vec2(hist_width, hist_height),
                    );

                    // Background
                    ui.painter().rect_filled(
                        hist_rect,
                        0.0,
                        egui::Color32::from_rgba_premultiplied(0, 0, 0, 180),
                    );

                    let max_count = self.histogram.max_count().max(1) as f32;
                    let bar_width = hist_width / 256.0;
                    let log_max = (max_count + 1.0).ln();

                    // Draw RGB histograms overlaid with logarithmic scaling
                    for i in 0..256 {
                        let x = hist_rect.left() + i as f32 * bar_width;
                        let bar_width_adj = bar_width.max(1.0);

                        // Red channel - use log scale and minimum 1px for visibility
                        if self.histogram.red[i] > 0 {
                            let red_count = self.histogram.red[i] as f32;
                            let red_height =
                                ((red_count + 1.0).ln() / log_max * hist_height).max(1.0);
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    egui::pos2(x, hist_rect.bottom() - red_height),
                                    egui::vec2(bar_width_adj, red_height),
                                ),
                                0.0,
                                egui::Color32::from_rgba_premultiplied(255, 0, 0, 128),
                            );
                        }

                        // Green channel
                        if self.histogram.green[i] > 0 {
                            let green_count = self.histogram.green[i] as f32;
                            let green_height =
                                ((green_count + 1.0).ln() / log_max * hist_height).max(1.0);
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    egui::pos2(x, hist_rect.bottom() - green_height),
                                    egui::vec2(bar_width_adj, green_height),
                                ),
                                0.0,
                                egui::Color32::from_rgba_premultiplied(0, 255, 0, 128),
                            );
                        }

                        // Blue channel
                        if self.histogram.blue[i] > 0 {
                            let blue_count = self.histogram.blue[i] as f32;
                            let blue_height =
                                ((blue_count + 1.0).ln() / log_max * hist_height).max(1.0);
                            ui.painter().rect_filled(
                                egui::Rect::from_min_size(
                                    egui::pos2(x, hist_rect.bottom() - blue_height),
                                    egui::vec2(bar_width_adj, blue_height),
                                ),
                                0.0,
                                egui::Color32::from_rgba_premultiplied(0, 0, 255, 128),
                            );
                        }
                    }
                }
            } else {
                ui.heading("Preview");
                ui.label("Add images and click 'Create HDR'");
            }
        });

        if self.is_loading || self.is_generating || self.rx.is_some() {
            ctx.request_repaint();
        }
    }
}

fn load_about_image() -> Option<image::DynamicImage> {
    let paths_to_try = vec![
        std::path::PathBuf::from("resources/sample.png"),
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(|p| p.join("resources/sample.png")))
            .unwrap_or_default(),
    ];

    for about_path in paths_to_try {
        if about_path.exists() {
            match image::open(&about_path) {
                Ok(img) => return Some(img),
                Err(e) => {
                    log::warn!("Could not load about image from {:?}: {}", about_path, e);
                }
            }
        }
    }
    None
}

pub fn run_gui() -> Result<(), HdrError> {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([600.0, 400.0]),
        ..Default::default()
    };

    eframe::run_native(
        "HDR-Oxide",
        options,
        Box::new(|_cc| Ok(Box::new(HdrApp::default()))),
    )
    .map_err(|e| HdrError::InvalidInput(format!("GUI error: {}", e)))
}
