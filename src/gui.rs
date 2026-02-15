use crate::error::HdrError;
use crate::image::merge::{merge_to_hdr_parallel, HdrImage};
use crate::image::tonemap::{tonemap_hdr, TonemapSettings};
use eframe::egui;
use image::GenericImageView;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;

const THUMBNAIL_SIZE: u32 = 80;

pub struct Thumbnail {
    pub path: PathBuf,
    pub texture: egui::TextureHandle,
}

pub enum GuiCommand {
    MergeComplete(Result<HdrImage, String>),
    LoadError(String),
    FilePickerComplete(Vec<PathBuf>),
    SaveComplete(Result<(), String>),
    ThumbnailsComplete(Vec<(PathBuf, image::RgbaImage)>),
    PreviewComplete(Result<image::RgbaImage, String>),
}

pub struct HdrApp {
    input_paths: Vec<PathBuf>,
    thumbnails: Vec<Thumbnail>,
    pending_thumbnail_paths: Vec<PathBuf>,
    hdr_image: Option<HdrImage>,
    preview_texture: Option<egui::TextureHandle>,
    tonemap_method: String,
    // Image adjustment controls
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
    is_processing: bool,
    rx: Option<mpsc::Receiver<GuiCommand>>,
    path_input: String,
    // About dialog
    show_about: bool,
    about_texture: Option<egui::TextureHandle>,
}

impl Default for HdrApp {
    fn default() -> Self {
        Self {
            input_paths: Vec::new(),
            thumbnails: Vec::new(),
            pending_thumbnail_paths: Vec::new(),
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
            is_processing: false,
            rx: None,
            path_input: String::new(),
            show_about: false,
            about_texture: None,
        }
    }
}

impl HdrApp {
    fn add_path(&mut self) {
        let input = &self.path_input;

        // Check if input contains wildcards
        if input.contains('*') || input.contains('?') {
            // Handle glob pattern
            match glob::glob(input) {
                Ok(paths) => {
                    let valid_exts = ["jpg", "jpeg", "png", "tif", "tiff"];
                    let mut added = 0;

                    for path in paths.flatten() {
                        if path.is_file() {
                            let ext = path
                                .extension()
                                .and_then(|e| e.to_str())
                                .unwrap_or("")
                                .to_lowercase();
                            if valid_exts.contains(&ext.as_str())
                                && !self.input_paths.contains(&path)
                            {
                                self.input_paths.push(path.clone());
                                self.pending_thumbnail_paths.push(path);
                                added += 1;
                            }
                        }
                    }

                    if added > 0 {
                        self.status = format!(
                            "{} images (added {} from pattern)",
                            self.input_paths.len(),
                            added
                        );
                        self.start_thumbnail_loading();
                    } else {
                        self.status = "No matching image files found".to_string();
                    }
                }
                Err(e) => {
                    self.status = format!("Invalid pattern: {}", e);
                }
            }
        } else {
            // Handle single file path
            let path = PathBuf::from(input);
            if path.exists() && path.is_file() {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if ["jpg", "jpeg", "png", "tif", "tiff"].contains(&ext.as_str()) {
                    if !self.input_paths.contains(&path) {
                        self.input_paths.push(path.clone());
                        self.status = format!("{} images", self.input_paths.len());
                        self.pending_thumbnail_paths.push(path);
                        self.start_thumbnail_loading();
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
        let mut added = 0;

        for path in paths {
            if path.exists() && path.is_file() {
                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if valid_exts.contains(&ext.as_str()) && !self.input_paths.contains(path) {
                    self.input_paths.push(path.clone());
                    self.pending_thumbnail_paths.push(path.clone());
                    added += 1;
                }
            }
        }

        if added > 0 {
            self.status = format!("{} images (added {})", self.input_paths.len(), added);
            // Don't start loading immediately if we're still processing the file picker
            // The caller should trigger thumbnail loading after clearing the receiver
        } else {
            self.status = "No valid image files selected".to_string();
        }
    }

    fn start_thumbnail_loading(&mut self) {
        if self.pending_thumbnail_paths.is_empty() {
            return;
        }

        if self.rx.is_some() {
            // Already processing something, don't start thumbnails yet
            // They will be loaded when current operation completes
            return;
        }

        self.load_thumbnails_async();
    }

    fn load_thumbnails_async(&mut self) {
        if self.pending_thumbnail_paths.is_empty() {
            return;
        }

        let paths_to_load = std::mem::take(&mut self.pending_thumbnail_paths);

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        thread::spawn(move || {
            let mut results = Vec::new();

            for path in paths_to_load {
                match load_thumbnail(&path) {
                    Some(image) => {
                        results.push((path, image));
                    }
                    None => {
                        log::warn!("Failed to load thumbnail for {:?}", path);
                    }
                }
            }

            let _ = tx.send(GuiCommand::ThumbnailsComplete(results));
        });
    }

    fn remove_thumbnail(&mut self, path: &PathBuf) {
        self.thumbnails.retain(|t| t.path != *path);
    }

    fn clear_images(&mut self) {
        self.input_paths.clear();
        self.thumbnails.clear();
        self.pending_thumbnail_paths.clear();
        self.hdr_image = None;
        self.preview_texture = None;
        self.status = "Cleared".to_string();
    }

    fn open_file_dialog(&mut self) {
        if self.is_processing {
            return;
        }

        self.is_processing = true;
        self.status = "Opening file dialog...".to_string();

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        thread::spawn(move || {
            let files = rfd::FileDialog::new()
                .add_filter("Images", &["jpg", "jpeg", "png", "tif", "tiff"])
                .set_title("Select Images for HDR")
                .pick_files();

            match files {
                Some(paths) => {
                    let _ = tx.send(GuiCommand::FilePickerComplete(paths));
                }
                None => {
                    let _ = tx.send(GuiCommand::FilePickerComplete(Vec::new()));
                }
            }
        });
    }

    fn start_merge(&mut self) {
        if self.input_paths.is_empty() {
            self.status = "No images".to_string();
            return;
        }

        let paths = self.input_paths.clone();
        self.is_processing = true;
        self.status = "Processing...".to_string();

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        thread::spawn(move || {
            use crate::image::loader::load_source_images;

            match load_source_images(&paths, None, None) {
                Ok(images) => match merge_to_hdr_parallel(&images) {
                    Ok(hdr) => {
                        let _ = tx.send(GuiCommand::MergeComplete(Ok(hdr)));
                    }
                    Err(e) => {
                        let _ =
                            tx.send(GuiCommand::MergeComplete(Err(format!("HDR failed: {}", e))));
                    }
                },
                Err(e) => {
                    let _ = tx.send(GuiCommand::LoadError(format!("Load failed: {}", e)));
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
                    self.hdr_image = Some(hdr);
                    self.status = "HDR ready".to_string();
                    self.is_processing = false;
                    self.rx = None;
                    // Note: update_preview is now async, so we pass ctx and it will handle itself
                    self.update_preview(ctx);
                }
                GuiCommand::MergeComplete(Err(e)) => {
                    self.status = e;
                    self.is_processing = false;
                    self.rx = None;
                    self.start_thumbnail_loading();
                }
                GuiCommand::LoadError(e) => {
                    self.status = e;
                    self.is_processing = false;
                    self.rx = None;
                    self.start_thumbnail_loading();
                }
                GuiCommand::FilePickerComplete(paths) => {
                    if !paths.is_empty() {
                        self.add_multiple_paths(&paths);
                    } else {
                        self.status = "No files selected".to_string();
                    }
                    self.is_processing = false;
                    self.rx = None;
                    // Start loading thumbnails now that receiver is cleared
                    self.start_thumbnail_loading();
                }
                GuiCommand::SaveComplete(result) => {
                    match result {
                        Ok(()) => self.status = "Saved successfully".to_string(),
                        Err(e) => self.status = format!("Save failed: {}", e),
                    }
                    self.is_processing = false;
                    self.rx = None;
                    self.start_thumbnail_loading();
                }
                GuiCommand::ThumbnailsComplete(images) => {
                    for (path, image) in images {
                        let (width, height) = image.dimensions();
                        let pixels: Vec<egui::Color32> = image
                            .pixels()
                            .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                            .collect();

                        let color_image = egui::ColorImage {
                            size: [width as usize, height as usize],
                            pixels,
                        };

                        let texture = ctx.load_texture(
                            format!(
                                "thumb_{}",
                                path.file_name().unwrap_or_default().to_string_lossy()
                            ),
                            Arc::new(color_image),
                            egui::TextureOptions::LINEAR,
                        );

                        self.thumbnails.push(Thumbnail { path, texture });
                    }
                    self.rx = None;
                    // Check if there are more thumbnails pending (e.g., added while we were loading)
                    self.start_thumbnail_loading();
                }
                GuiCommand::PreviewComplete(result) => {
                    match result {
                        Ok(image) => {
                            self.apply_preview_texture(ctx, image);
                        }
                        Err(e) => {
                            self.status = format!("Tonemap failed: {}", e);
                        }
                    }
                    self.is_processing = false;
                    self.rx = None;
                    self.start_thumbnail_loading();
                }
            }
        }
    }

    fn update_preview(&mut self, _ctx: &egui::Context) {
        if self.hdr_image.is_none() || self.is_processing {
            return;
        }

        self.is_processing = true;
        self.status = "Updating preview...".to_string();

        let hdr_clone = HdrImage {
            data: self.hdr_image.as_ref().unwrap().data.clone(),
            width: self.hdr_image.as_ref().unwrap().width,
            height: self.hdr_image.as_ref().unwrap().height,
        };
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

        thread::spawn(
            move || match tonemap_hdr(&hdr_clone, &tonemap_method, &settings) {
                Ok(tonemapped) => {
                    let rgba = tonemapped.to_rgba8();
                    let _ = tx.send(GuiCommand::PreviewComplete(Ok(rgba)));
                }
                Err(e) => {
                    let _ = tx.send(GuiCommand::PreviewComplete(Err(e.to_string())));
                }
            },
        );
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

    fn save_output(&mut self) {
        if self.hdr_image.is_none() || self.is_processing {
            return;
        }

        self.is_processing = true;
        self.status = "Opening save dialog...".to_string();

        let hdr_clone = HdrImage {
            data: self.hdr_image.as_ref().unwrap().data.clone(),
            width: self.hdr_image.as_ref().unwrap().width,
            height: self.hdr_image.as_ref().unwrap().height,
        };
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

        thread::spawn(move || {
            let file = rfd::FileDialog::new()
                .add_filter("PNG", &["png"])
                .add_filter("JPEG", &["jpg", "jpeg"])
                .set_title("Save HDR")
                .set_file_name("hdr_output.png")
                .save_file();

            match file {
                Some(path) => match tonemap_hdr(&hdr_clone, &tonemap_method, &settings) {
                    Ok(tonemapped) => match tonemapped.save(&path) {
                        Ok(()) => {
                            let _ = tx.send(GuiCommand::SaveComplete(Ok(())));
                        }
                        Err(e) => {
                            let _ = tx.send(GuiCommand::SaveComplete(Err(e.to_string())));
                        }
                    },
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
                if ui.button("Open Files...").clicked() && !self.is_processing {
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

                    // Display thumbnails vertically, full width
                    let panel_width = ui.available_width();
                    let to_remove = egui::ScrollArea::vertical()
                        .max_height(300.0)
                        .show(ui, |ui| {
                            let mut remove_idx = None;
                            let spacing = 4.0;

                            for (i, path) in self.input_paths.iter().enumerate() {
                                ui.vertical(|ui| {
                                    ui.set_width(panel_width);

                                    // Thumbnail image - full panel width
                                    if let Some(thumb) =
                                        self.thumbnails.iter().find(|t| t.path == *path)
                                    {
                                        let size = thumb.texture.size();
                                        let aspect = size[1] as f32 / size[0] as f32;
                                        let height = (panel_width - spacing * 2.0) * aspect;
                                        let max_height = 120.0f32; // Cap height for very wide images
                                        let final_height = height.min(max_height);

                                        ui.add(egui::Image::new(&thumb.texture).fit_to_exact_size(
                                            egui::vec2(panel_width - spacing * 2.0, final_height),
                                        ));
                                    } else {
                                        ui.add_sized(
                                            egui::vec2(panel_width - spacing * 2.0, 80.0),
                                            egui::Spinner::new().size(24.0),
                                        );
                                    }

                                    // Filename and remove button in one row
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
                        self.remove_thumbnail(&path);
                        self.hdr_image = None;
                        self.preview_texture = None;
                    }

                    if ui.button("Clear All").clicked() {
                        self.clear_images();
                    }
                }
            });

            ui.separator();

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

            ui.add(egui::Slider::new(&mut self.exposure, 0.1..=10.0).text("Exposure"));
            ui.add(egui::Slider::new(&mut self.contrast, 0.0..=2.0).text("Contrast"));

            // Collapsible advanced controls
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

            if self.hdr_image.is_some()
                && !self.is_processing
                && ui.button("Update Preview").clicked()
            {
                self.update_preview(ctx);
            }

            if ui.button("Create HDR").clicked()
                && !self.input_paths.is_empty()
                && !self.is_processing
            {
                self.start_merge();
            }

            if ui.button("Save").clicked() && self.hdr_image.is_some() && !self.is_processing {
                self.save_output();
            }

            ui.separator();

            if ui.button("About").clicked() {
                self.show_about = true;
                // Load about image if not already loaded
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

            if self.is_processing {
                ui.add(egui::Spinner::new());
            }
            ui.label(&self.status);
        });

        // About dialog window
        if self.show_about {
            egui::Window::new("About HDR Oxide")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        // Show sample image if loaded
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
            ui.heading("Preview");

            if let Some(ref texture) = self.preview_texture {
                let size = texture.size();
                let available = ui.available_size();
                let scale = (available.x / size[0] as f32)
                    .min(available.y / size[1] as f32)
                    .min(1.0);
                let scaled_size = egui::vec2(size[0] as f32 * scale, size[1] as f32 * scale);

                ui.add(egui::Image::new(texture).max_size(scaled_size));
            } else {
                ui.label("Add images and click 'Create HDR'");
            }
        });
    }
}

fn load_thumbnail(path: &PathBuf) -> Option<image::RgbaImage> {
    match image::open(path) {
        Ok(img) => {
            let (width, height) = img.dimensions();
            let aspect = width as f32 / height as f32;

            let (new_width, new_height) = if aspect > 1.0 {
                (THUMBNAIL_SIZE, (THUMBNAIL_SIZE as f32 / aspect) as u32)
            } else {
                ((THUMBNAIL_SIZE as f32 * aspect) as u32, THUMBNAIL_SIZE)
            };

            let resized = img.resize(new_width, new_height, image::imageops::FilterType::Triangle);
            Some(resized.to_rgba8())
        }
        Err(e) => {
            log::error!("Failed to load image {:?}: {}", path, e);
            None
        }
    }
}

fn load_about_image() -> Option<image::DynamicImage> {
    let about_path = std::path::PathBuf::from("resources/sample.png");
    match image::open(&about_path) {
        Ok(img) => Some(img),
        Err(e) => {
            log::warn!("Could not load about image from {:?}: {}", about_path, e);
            None
        }
    }
}

pub fn run_gui() -> Result<(), HdrError> {
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
