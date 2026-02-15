# HDR-Oxide

A GUI and CLI application for creating HDR (High Dynamic Range) images from multiple exposure photographs.

## Features

- Load multiple images (JPEG, PNG, TIFF)
- Automatic exposure time extraction from EXIF
- Manual exposure time or EV offset specification
- HDR merging using weighted luminance algorithm
- Tone-mapped preview output (Reinhard, Filmic, Gamma methods)
- Image alignment using ORB feature matching + homography (for handheld shots, optional feature)

> **Note:** For typical HDR (tripod/exposure bracketing), use `--no-align`. Alignment is only needed for handheld shots where the camera position shifted between exposures.

## Installation

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

The alignment feature uses ORB (Oriented FAST and Rotated BRIEF) feature detection with homography to align handheld bracketed exposures.

On Ubuntu 24.04:
```bash
sudo apt-get install libclang-dev libstdc++-13-dev libopencv-dev
```

## Usage

```bash
# Create HDR from images (auto-detect exposures from EXIF)
hdr-oxide create -i img1.jpg img2.jpg img3.jpg -o output.tiff

# With tone-mapped preview
hdr-oxide create -i *.jpg -o hdr.tiff --preview preview.jpg

# Manual exposure times
hdr-oxide create -i img1.jpg img2.jpg -e 1/1000 1/125 -o hdr.tiff

# EV offsets (in stops)
hdr-oxide create -i *.jpg -o hdr.tiff --ev-offsets 0,3,7

# Skip alignment (for tripod shots)
hdr-oxide create -i *.jpg -o hdr.tiff --no-align

# Specify tonemap method
hdr-oxide create -i *.jpg -o hdr.tiff --preview preview.jpg --tonemap-method filmic

# Adjust exposure for tonemapping
hdr-oxide create -i *.jpg -o hdr.tiff --preview preview.jpg --exposure-adjust 1.5

# Verbose logging
hdr-oxide create -i *.jpg -o hdr.tiff --verbose
```

### Options

| Option | Description |
|--------|-------------|
| `-i, --input` | Input image files (JPEG, PNG, TIFF) |
| `-o, --output` | Output HDR file (32-bit float TIFF) |
| `-p, --preview` | Output tone-mapped preview image |
| `--no-align` | Skip image alignment (use for pre-aligned tripod shots) |
| `-e, --exposure` | Manual exposure times (e.g., 1/1000 1/125 1/15) |
| `--ev-offsets` | Exposure value offsets in stops (e.g., 0,3,7) |
| `--tonemap-method` | Tonemap method: reinhard, filmic, gamma (default: reinhard) |
| `--exposure-adjust` | Exposure adjustment for tonemapping (default: 1.0) |
| `--verbose` | Enable verbose logging |

## Commands

### create

Create an HDR image from multiple source images.

```bash
hdr-oxide create -i img1.jpg img2.jpg img3.jpg -o output.tiff
```

### info

Display information about an HDR image.

```bash
hdr-oxide info input.tiff
```

## How It Works

1. **Image Loading**: Reads input images and extracts EXIF metadata for exposure times
2. **Alignment** (optional, with `--features alignment`): Uses ORB feature detection + BFMatcher + homography to align misaligned images to the first exposure
3. **HDR Merging**: Combines images using weighted luminance based on relative exposures
4. **Output**: Saves as 32-bit float TIFF with optional tone-mapped preview

## Building

### Development Build

```bash
cargo build
```

### With Alignment

```bash
CPATH=/usr/include/c++/13:/usr/include/x86_64-linux-gnu/c++/13 cargo build --features alignment
```

### Release Build

```bash
cargo build --release
```

## License

MIT
