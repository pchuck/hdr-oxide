use clap::{Parser, ValueHint};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "hdr-oxide")]
#[command(version = "0.1.0")]
#[command(about = "Create HDR images from multiple exposure photographs", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Parser, Debug)]
pub enum Commands {
    #[command(about = "Create an HDR image from multiple source images")]
    Create(CreateArgs),
    #[command(about = "Display information about an HDR image")]
    Info(InfoArgs),
    #[command(about = "Open GUI for HDR creation")]
    Gui,
}

#[derive(Parser, Debug)]
#[command(about = "Create an HDR image from multiple source images")]
pub struct CreateArgs {
    #[arg(short, long, required = true, num_args = 1.., value_hint = ValueHint::FilePath)]
    #[arg(help = "Input image files (JPEG, PNG, TIFF, etc.)")]
    pub input: Vec<PathBuf>,

    #[arg(short, long, required = true, value_hint = ValueHint::FilePath)]
    #[arg(help = "Output image file (PNG or JPEG)")]
    pub output: PathBuf,

    #[arg(long, default_value_t = false)]
    #[arg(help = "Skip image alignment (use if images are already aligned)")]
    pub no_align: bool,

    #[arg(long, default_value_t = false)]
    #[arg(help = "Enable verbose logging")]
    pub verbose: bool,

    #[arg(short, long, num_args = 1..)]
    #[arg(
        help = "Exposure times in seconds (e.g., 1/1000 1/125 1/15). Auto-detected from EXIF if not provided"
    )]
    pub exposure: Option<Vec<String>>,

    #[arg(long)]
    #[arg(help = "Exposure value offset for each image (comma-separated, e.g., 0,3,7)")]
    pub ev_offsets: Option<Vec<i32>>,

    #[arg(long, default_value = "reinhard")]
    #[arg(help = "Tonemapping method: reinhard, filmic, gamma")]
    pub tonemap_method: String,

    #[arg(long, default_value_t = 1.0)]
    #[arg(help = "Exposure adjustment for tonemapping")]
    pub exposure_adjust: f32,

    #[arg(long, default_value_t = 1.0)]
    #[arg(help = "Contrast adjustment for tonemapping (1.0 = neutral)")]
    pub contrast: f32,

    #[arg(long, default_value_t = 1.0)]
    #[arg(help = "Saturation adjustment (1.0 = neutral)")]
    pub saturation: f32,

    #[arg(long, default_value_t = 0.0)]
    #[arg(help = "Vibrance adjustment (-100 to 100)")]
    pub vibrance: f32,

    #[arg(long, default_value_t = 0.0)]
    #[arg(help = "Shadows lift adjustment (-100 to 100)")]
    pub shadows: f32,

    #[arg(long, default_value_t = 0.0)]
    #[arg(help = "Highlights compression adjustment (-100 to 100)")]
    pub highlights: f32,

    #[arg(long, default_value_t = 0.0)]
    #[arg(help = "Color temperature adjustment (-100 = cooler, 100 = warmer)")]
    pub temperature: f32,

    #[arg(long, default_value_t = 0.0)]
    #[arg(help = "Color tint adjustment (-100 = green, 100 = magenta)")]
    pub tint: f32,

    #[arg(long, default_value_t = 0.0)]
    #[arg(help = "Hue shift in degrees (-180 to 180)")]
    pub hue_shift: f32,

    #[arg(long, default_value_t = 0.0)]
    #[arg(help = "Sharpen/blur amount (-100 = blur, 0 = neutral, 100 = sharpen)")]
    pub sharpen: f32,
}

#[derive(Parser, Debug)]
#[command(about = "Display information about an HDR image")]
pub struct InfoArgs {
    #[arg(required = true, value_hint = ValueHint::FilePath)]
    #[arg(help = "HDR image file to inspect")]
    pub input: PathBuf,

    #[arg(short, long, default_value_t = false)]
    #[arg(help = "Show detailed channel information")]
    pub verbose: bool,
}
