# HDR-Oxide

A GUI and CLI application for creating HDR (High Dynamic Range) images from multiple exposure photographs built with Rust, eframe/egui and rayon for parallel rendering.

![HDR Oxide Screenshot](resources/screenshot.png)

## Features

- **GUI Mode**: Interactive graphical interface with real-time preview
- **Multiple Input Formats**: JPEG, PNG, TIFF support
- **Automatic Exposure Detection**: Extracts exposure times from EXIF metadata
- **Manual Exposure Control**: Specify exposure times or EV offsets manually
- **HDR Merging**: Weighted luminance algorithm for combining exposures
- **Image Alignment**: ORB feature matching + homography for handheld shots (optional)
- **Advanced Tone Mapping**: Reinhard, Filmic, and Gamma methods
- **Image Adjustments**: Exposure, contrast, saturation, vibrance, shadows, highlights, temperature, tint, hue shift, and sharpening/blur

> **Note:** For typical HDR (tripod/exposure bracketing), use `--no-align`. Alignment is only needed for handheld shots where the camera position shifted between exposures.

## Installation

### Basic Build

```bash
cargo build --release
```

### With Alignment Support

For OpenCV-based image alignment:

```bash
# Install system dependencies
sudo apt-get install libclang-dev libstdc++-13-dev

# Build with alignment feature
CPATH=/usr/include/c++/13:/usr/include/x86_64-linux-gnu/c++/13 cargo build --features alignment --release
```

**Prerequisites for OpenCV:**
- `libclang-dev` - Required for Rust OpenCV bindings
- `libstdc++-13-dev` - C++ standard library headers
- OpenCV 4.x development libraries

On Ubuntu 24.04:
```bash
sudo apt-get install libclang-dev libstdc++-13-dev libopencv-dev
```

## Usage

### GUI Mode

Launch the interactive GUI:

```bash
hdr-oxide gui
```

The GUI provides:
- Visual thumbnail management
- Real-time preview with adjustment controls
- Advanced tone mapping controls
- One-click HDR creation

### CLI Mode

#### Basic Usage

```bash
# Create HDR from images (auto-detect exposures from EXIF)
hdr-oxide create -i img1.jpg img2.jpg img3.jpg -o output.tiff

# Process multiple files with wildcards
hdr-oxide create -i *.jpg -o hdr.tiff

# Skip alignment (recommended for tripod shots)
hdr-oxide create -i *.jpg -o hdr.tiff --no-align

# Verbose output
hdr-oxide create -i *.jpg -o hdr.tiff --verbose
```

#### Exposure Control

```bash
# Manual exposure times (in seconds)
hdr-oxide create -i img1.jpg img2.jpg -e 1/1000 1/125 -o hdr.tiff

# EV offsets in stops
hdr-oxide create -i *.jpg -o hdr.tiff --ev-offsets 0,3,7
```

#### Tone Mapping

```bash
# Specify tonemap method
hdr-oxide create -i *.jpg -o hdr.tiff --tonemap-method filmic

# Available methods: reinhard, filmic, gamma

# Adjust exposure for tonemapping
hdr-oxide create -i *.jpg -o hdr.tiff --exposure-adjust 1.5

# Adjust contrast (1.0 = neutral)
hdr-oxide create -i *.jpg -o hdr.tiff --contrast 1.2
```

#### Advanced Image Adjustments

```bash
# Saturation (0.0-2.0, 1.0 = neutral)
hdr-oxide create -i *.jpg -o hdr.tiff --saturation 1.2

# Vibrance (-100 to 100, protects skin tones)
hdr-oxide create -i *.jpg -o hdr.tiff --vibrance 20

# Shadows lift (-100 to 100)
hdr-oxide create -i *.jpg -o hdr.tiff --shadows 30

# Highlights compression (-100 to 100)
hdr-oxide create -i *.jpg -o hdr.tiff --highlights -20

# Color temperature (-100 = cooler, 100 = warmer)
hdr-oxide create -i *.jpg -o hdr.tiff --temperature 10

# Color tint (-100 = green, 100 = magenta)
hdr-oxide create -i *.jpg -o hdr.tiff --tint 5

# Hue shift (-180 to 180 degrees)
hdr-oxide create -i *.jpg -o hdr.tiff --hue-shift 15

# Sharpen/Blur (-100 = blur, 0 = neutral, 100 = sharpen)
hdr-oxide create -i *.jpg -o hdr.tiff --sharpen 25
```

### Info Command

Display information about an HDR image:

```bash
hdr-oxide info input.tiff
hdr-oxide info input.tiff --verbose
```

## CLI Options

| Option | Description |
|--------|-------------|
| `-i, --input` | Input image files (JPEG, PNG, TIFF) |
| `-o, --output` | Output HDR file (32-bit float TIFF) |
| `--no-align` | Skip image alignment (use for pre-aligned tripod shots) |
| `-e, --exposure` | Manual exposure times (e.g., 1/1000 1/125 1/15) |
| `--ev-offsets` | Exposure value offsets in stops (e.g., 0,3,7) |
| `--tonemap-method` | Tonemap method: reinhard, filmic, gamma (default: reinhard) |
| `--exposure-adjust` | Exposure adjustment for tonemapping (default: 1.0) |
| `--contrast` | Contrast adjustment (default: 1.0) |
| `--saturation` | Saturation adjustment (default: 1.0) |
| `--vibrance` | Vibrance adjustment (default: 0) |
| `--shadows` | Shadows lift (default: 0) |
| `--highlights` | Highlights compression (default: 0) |
| `--temperature` | Color temperature (default: 0) |
| `--tint` | Color tint (default: 0) |
| `--hue-shift` | Hue rotation in degrees (default: 0) |
| `--sharpen` | Sharpen/blur amount (default: 0) |
| `--verbose` | Enable verbose logging |

## Building

### Development Build

```bash
cargo build
```

### With Alignment Support

```bash
CPATH=/usr/include/c++/13:/usr/include/x86_64-linux-gnu/c++/13 cargo build --features alignment
```

### Release Build

```bash
cargo build --release
```

### Run GUI

```bash
cargo run -- gui
```

## How It Works

1. **Image Loading**: Reads input images and extracts EXIF metadata for exposure times
2. **Alignment** (optional, with `--features alignment`): Uses ORB feature detection + BFMatcher + homography to align misaligned images to the first exposure
3. **HDR Merging**: Combines images using weighted luminance based on relative exposures with parallel processing
4. **Tone Mapping**: Applies selected tone mapping algorithm with user-adjustable parameters
5. **Output**: Saves as 32-bit float TIFF

## License

MIT

## Copyright

Copyright © 2026 ultrametrics
