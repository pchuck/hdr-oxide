# AGENTS.md - HDR-Oxide Project Guidelines

## Project Overview

HDR-Oxide is a Rust application for creating HDR images from exposure-bracketed photographs. It provides both a CLI and a GUI (egui) interface with tonemapping capabilities.

## Build Commands

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run GUI
cargo run -- gui

# Run CLI
cargo run -- create -i img1.jpg -i img2.jpg -o output.png
```

## Test Commands

```bash
# Run all tests
cargo test

# Run a single test
cargo test test_merge_single_image

# Run tests in a specific file
cargo test --test merge_tests

# Run tests matching a pattern
cargo test tonemap

# Run tests with output
cargo test -- --nocapture
```

## Lint Commands

```bash
# Run clippy (must pass with no warnings)
cargo clippy

# Run clippy with all warnings
cargo clippy -- -W clippy::all

# Format check
cargo fmt -- --check

# Auto-format
cargo fmt
```

## Code Style Guidelines

### Imports

```rust
// Order: std -> external crates -> internal modules
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use image::{DynamicImage, Rgba32FImage};
use rayon::prelude::*;

use crate::error::HdrError;
use crate::image::merge::HdrImage;
```

### Formatting

- Use `cargo fmt` before committing
- Max line length: 100 characters (default rustfmt)
- Indent: 4 spaces (no tabs)
- Match arm bodies align when short

### Types

- Use `f32` for image processing values (HDR data, color channels)
- Use `u32` for image dimensions, `u64` for dimension multiplication to avoid overflow
- Use `Arc<T>` for shared image data across threads
- Use `PathBuf` for file paths

### Naming Conventions

- **Modules**: `snake_case` (`merge.rs`, `tonemap.rs`)
- **Types/Structs**: `PascalCase` (`HdrImage`, `TonemapSettings`, `SourceImage`)
- **Functions**: `snake_case` (`merge_to_hdr`, `tonemap_hdr`)
- **Constants**: `SCREAMING_SNAKE_CASE` (`LUMINANCE_R`, `FILMIC_A`)
- **Local variables**: `snake_case`

### Error Handling

- Use `Result<T, HdrError>` for fallible operations
- Use `anyhow::Result` for CLI commands
- Always propagate errors with `?`, never panic in library code
- GUI uses `Result<T, String>` for thread communication

```rust
// Library code
pub fn merge_to_hdr(images: &[SourceImage]) -> Result<HdrImage, HdrError> {
    if images.is_empty() {
        return Err(HdrError::Merge("No images to merge".to_string()));
    }
    // ...
}

// CLI code
pub fn create_hdr(args: CreateArgs) -> Result<()> {
    let images = loader::load_source_images(&args.input, exposures, ev_offsets)?;
    // ...
}
```

### Threading and Concurrency

- Use `rayon` for parallel pixel processing: `(0..height).into_par_iter()`
- Use `std::sync::mpsc` for GUI thread communication
- Use `Arc<AtomicBool>` for cancellation tokens
- Use `Arc<T>` for shared image data

```rust
// Parallel processing pattern
let results: Vec<_> = (0..height)
    .into_par_iter()
    .flat_map(|y| (0..width).into_par_iter().map(move |x| (x, y)))
    .map(|(x, y)| {
        // process pixel
    })
    .collect();
```

### Progress Reporting

- Use callback pattern: `Fn(usize) + Send + Sync`
- Report progress at intervals (e.g., every 1% of pixels)
- Use `saturating_sub` for atomic counter differences

```rust
pub fn merge_to_hdr_parallel_with_progress<F>(
    images: &[SourceImage],
    progress_callback: F,
) -> Result<HdrImage, HdrError>
where
    F: Fn(usize) + Send + Sync,
{
    // ... report progress periodically
    progress_callback(count);
}
```

### Code Comments

- **Do NOT add comments** unless explicitly requested
- Named constants should be self-documenting
- Use descriptive function/variable names instead of comments

### Magic Numbers

- Define as named constants at module level

```rust
const LUMINANCE_R: f32 = 0.2126;
const LUMINANCE_G: f32 = 0.7152;
const LUMINANCE_B: f32 = 0.0722;
const EPSILON: f32 = 1e-6;
```

### Float Comparisons

- Never use `==` for float equality
- Use epsilon comparison: `(a - b).abs() < EPSILON`

```rust
// Correct
if delta < EPSILON { ... }
if (max - r).abs() < EPSILON { ... }

// Wrong
if delta == 0.0 { ... }
```

### NaN and Infinity Handling

- Always check for NaN after mathematical operations
- Use `is_finite()` for validation
- Provide fallback values for edge cases

```rust
fn apply_filmic_curve(x: f32) -> f32 {
    let result = // ... complex calculation
    if result.is_nan() {
        0.0
    } else {
        result
    }
}
```

### Test Patterns

- Tests go in `tests/` directory, not inline
- Create helper functions for test data
- Use `assert!((actual - expected).abs() < EPSILON)` for float comparisons

```rust
fn create_test_image(width: u32, height: u32, color: [u8; 3]) -> DynamicImage {
    let mut img = RgbImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            img.put_pixel(x, y, Rgb(color));
        }
    }
    DynamicImage::ImageRgb8(img)
}

#[test]
fn test_merge_single_image() {
    let img = create_test_image(10, 10, [128, 128, 128]);
    // ...
}
```

## Key Dependencies

- `image` - Image loading/saving, format handling
- `rayon` - Parallel iteration
- `eframe/egui` - GUI framework
- `clap` - CLI argument parsing
- `anyhow` - Error handling for CLI
- `kamadak-exif` - EXIF metadata reading
- `rfd` - File dialogs
- `indicatif` - CLI progress bars

## Architecture Notes

- **src/lib.rs**: Module exports
- **src/main.rs**: CLI entry point
- **src/cli.rs**: CLI argument definitions
- **src/gui.rs**: GUI application (HdrApp struct)
- **src/error.rs**: Error types
- **src/commands/mod.rs**: CLI command handlers
- **src/image/**: Image processing modules
  - **loader.rs**: Image loading, EXIF parsing
  - **merge.rs**: HDR merging (Debevec weighting)
  - **tonemap.rs**: Tone mapping (Reinhard, Filmic, Gamma)
  - **alignment.rs**: Image alignment (optional, requires OpenCV)

## Working Resolution Pattern

The GUI uses a working resolution (max 2048px) for interactive operations:

```rust
const WORKING_MAX_DIMENSION: u32 = 2048;

pub struct SourceImages {
    pub full: Arc<DynamicImage>,      // Full resolution
    pub working: Arc<DynamicImage>,   // Max 2048px
    pub exposure_seconds: f32,
}
```

## Git Workflow

- Never commit without explicit user request
- Never push to remote without explicit user request
- Always run `cargo clippy` and `cargo test` after changes
