.PHONY: help build build-align release release-align clean test check lint gui run run-align info install

# Default paths (override with: make run INPUT="img1.jpg img2.jpg" OUTPUT="output.tiff")
INPUT=input/*.jpeg
OUTPUT=output.tiff
FILE=output.tiff

help:
	@echo "HDR-Oxide Makefile"
	@echo ""
	@echo "Build targets:"
	@echo "  build         - Development build (without alignment)"
	@echo "  build-align   - Development build with OpenCV alignment support"
	@echo "  release       - Optimized release build (without alignment)"
	@echo "  release-align - Optimized release with alignment support"
	@echo ""
	@echo "Quality targets:"
	@echo "  check         - Run cargo check"
	@echo "  lint          - Run clippy lints"
	@echo "  test          - Run tests"
	@echo ""
	@echo "Run targets:"
	@echo "  gui           - Launch the GUI"
	@echo "  run           - Run CLI with default/sample images"
	@echo "  run-align     - Run with alignment enabled"
	@echo "  info          - Show HDR file info"
	@echo ""
	@echo "Maintenance:"
	@echo "  clean         - Clean build artifacts"
	@echo "  install       - Install to ~/.cargo/bin"
	@echo ""
	@echo "Examples:"
	@echo "  make gui"
	@echo "  make run INPUT='img1.jpg img2.jpg img3.jpg' OUTPUT='hdr.tiff'"
	@echo "  make release-align"

# Build targets
build:
	cargo build

build-align:
	CPATH=/usr/include/c++/13:/usr/include/x86_64-linux-gnu/c++/13 cargo build --features alignment

release:
	cargo build --release

release-align:
	CPATH=/usr/include/c++/13:/usr/include/x86_64-linux-gnu/c++/13 cargo build --features alignment --release

# Quality assurance
check:
	cargo check

lint:
	cargo clippy -- -D warnings

test:
	cargo test

# Run targets
gui: build
	./target/debug/hdr-oxide gui

gui-release: release
	./target/release/hdr-oxide gui

run: build
	./target/debug/hdr-oxide create -i $(INPUT) -o $(OUTPUT)

run-align: build-align
	./target/debug/hdr-oxide create -i $(INPUT) -o $(OUTPUT)

run-release: release
	./target/release/hdr-oxide create -i $(INPUT) -o $(OUTPUT)

info: build
	./target/debug/hdr-oxide info $(FILE)

# Maintenance
clean:
	cargo clean
	@echo "Cleaned build artifacts"

install: release
	cargo install --path . --force

# Development helpers
fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

doc:
	cargo doc --open
