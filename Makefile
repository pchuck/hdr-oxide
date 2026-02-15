.PHONY: help build build-align release release-align clean test run info

INPUT=input/*.jpeg
OUTPUT=output.tiff

help:
	@echo "HDR-Oxide Makefile"
	@echo ""
	@echo "Available targets:"
	@echo "  build         - Build the CLI (without alignment)"
	@echo "  build-align  - Build with OpenCV alignment support"
	@echo "  release      - Release build (without alignment)"
	@echo "  release-align - Release build with alignment support"
	@echo "  clean        - Clean build artifacts"
	@echo "  test         - Run tests"
	@echo "  run          - Run with sample images (edit paths first)"
	@echo "  info         - Show HDR file info"

build:
	cargo build

build-align:
	CPATH=/usr/include/c++/13:/usr/include/x86_64-linux-gnu/c++/13 cargo build --features alignment

release:
	cargo build --release

release-align:
	CPATH=/usr/include/c++/13:/usr/include/x86_64-linux-gnu/c++/13 cargo build --features alignment --release

clean:
	cargo clean

test:
	cargo test

run:
	@echo "Usage: make run INPUT='img1.jpg img2.jpg' OUTPUT='hdr.tiff'"
	@echo "Edit the Makefile to set default paths"
	./target/debug/hdr-oxide create -i $(INPUT) -o $(OUTPUT)

info:
	./target/debug/hdr-oxide info $(FILE)
