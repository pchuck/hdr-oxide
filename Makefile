.PHONY: help build build-align release release-align clean test check lint gui run run-align info install

# Distribution targets
DIST_DIR := dist
APP_NAME := HDR-Oxide
APP_VERSION := 0.1.0

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
	@echo "Distribution targets:"
	@echo "  dist-mac      - Build macOS .dmg package"
	@echo "  dist-linux    - Build Linux .deb package"
	@echo "  dist-windows  - Build Windows ZIP distribution"
	@echo "  dist          - Build all distributions"
	@echo ""
	@echo "Maintenance:"
	@echo "  clean         - Clean build artifacts"
	@echo "  install       - Install to ~/.cargo/bin"
	@echo ""
	@echo "Examples:"
	@echo "  make gui"
	@echo "  make run INPUT='img1.jpg img2.jpg img3.jpg' OUTPUT='hdr.tiff'"
	@echo "  make release-align"
	@echo "  make dist-mac"

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

# Distribution targets
dist-clean:
	rm -rf $(DIST_DIR)
	mkdir -p $(DIST_DIR)

dist-mac: build release dist-clean
	@echo "Building macOS .dmg package..."
	mkdir -p $(DIST_DIR)/mac/$(APP_NAME).app/Contents/MacOS
	mkdir -p $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Resources
	cp target/release/hdr-oxide $(DIST_DIR)/mac/$(APP_NAME).app/Contents/MacOS/$(APP_NAME)
	cp LICENSE $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Resources/
	@echo '<?xml version="1.0" encoding="UTF-8"?>' > $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '<plist version="1.0">' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '<dict>' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '    <key>CFBundleName</key>' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '    <string>$(APP_NAME)</string>' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '    <key>CFBundleDisplayName</key>' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '    <string>HDR Oxide</string>' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '    <key>CFBundleIdentifier</key>' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '    <string>net.ultrametrics.hdr-oxide</string>' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '    <key>CFBundleVersion</key>' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '    <string>$(APP_VERSION)</string>' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '    <key>CFBundleExecutable</key>' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '    <string>$(APP_NAME)</string>' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '    <key>LSMinimumSystemVersion</key>' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '    <string>10.11</string>' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '</dict>' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo '</plist>' >> $(DIST_DIR)/mac/$(APP_NAME).app/Contents/Info.plist
	@echo "Creating .dmg file..."
	@which hdiutil > /dev/null 2>&1 && hdiutil create -volname "$(APP_NAME)" -srcfolder $(DIST_DIR)/mac -ov -format UDZO $(DIST_DIR)/$(APP_NAME)-$(APP_VERSION)-macOS.dmg || echo "Note: hdiutil not available (requires macOS). App bundle created in $(DIST_DIR)/mac/"
	@echo "macOS distribution created: $(DIST_DIR)/$(APP_NAME)-$(APP_VERSION)-macOS.dmg"

dist-linux: build release dist-clean
	@echo "Building Linux .deb package..."
	mkdir -p $(DIST_DIR)/linux/usr/bin
	mkdir -p $(DIST_DIR)/linux/usr/share/applications
	mkdir -p $(DIST_DIR)/linux/usr/share/doc/hdr-oxide
	mkdir -p $(DIST_DIR)/linux/DEBIAN
	cp target/release/hdr-oxide $(DIST_DIR)/linux/usr/bin/
	cp LICENSE $(DIST_DIR)/linux/usr/share/doc/hdr-oxide/copyright
	@echo "[Desktop Entry]" > $(DIST_DIR)/linux/usr/share/applications/hdr-oxide.desktop
	@echo "Name=HDR Oxide" >> $(DIST_DIR)/linux/usr/share/applications/hdr-oxide.desktop
	@echo "Comment=HDR image creation from bracketed photographs" >> $(DIST_DIR)/linux/usr/share/applications/hdr-oxide.desktop
	@echo "Exec=hdr-oxide" >> $(DIST_DIR)/linux/usr/share/applications/hdr-oxide.desktop
	@echo "Type=Application" >> $(DIST_DIR)/linux/usr/share/applications/hdr-oxide.desktop
	@echo "Categories=Graphics;PhotoProcessing;" >> $(DIST_DIR)/linux/usr/share/applications/hdr-oxide.desktop
	@echo "Package: hdr-oxide" > $(DIST_DIR)/linux/DEBIAN/control
	@echo "Version: $(APP_VERSION)" >> $(DIST_DIR)/linux/DEBIAN/control
	@echo "Section: graphics" >> $(DIST_DIR)/linux/DEBIAN/control
	@echo "Priority: optional" >> $(DIST_DIR)/linux/DEBIAN/control
	@echo "Architecture: amd64" >> $(DIST_DIR)/linux/DEBIAN/control
	@echo "Depends: libc6 (>= 2.17), libgcc1 (>= 1:4.1.1)" >> $(DIST_DIR)/linux/DEBIAN/control
	@echo "Maintainer: ultrametrics.net <contact@ultrametrics.net>" >> $(DIST_DIR)/linux/DEBIAN/control
	@echo "Description: HDR image creation from exposure-bracketed photographs" >> $(DIST_DIR)/linux/DEBIAN/control
	@echo " Built with Rust, supports GUI and CLI interfaces." >> $(DIST_DIR)/linux/DEBIAN/control
	@echo " Features include tonemapping, alignment, and EXR format support." >> $(DIST_DIR)/linux/DEBIAN/control
	@which dpkg-deb > /dev/null 2>&1 && dpkg-deb --build $(DIST_DIR)/linux $(DIST_DIR)/hdr-oxide_$(APP_VERSION)_amd64.deb || echo "Note: dpkg-deb not available. Package structure created in $(DIST_DIR)/linux/"
	@echo "Linux distribution created: $(DIST_DIR)/hdr-oxide_$(APP_VERSION)_amd64.deb"

dist-windows: build release dist-clean
	@echo "Building Windows distribution..."
	mkdir -p $(DIST_DIR)/windows
	cp target/release/hdr-oxide.exe $(DIST_DIR)/windows/HDR-Oxide.exe
	cp LICENSE $(DIST_DIR)/windows/LICENSE.txt
	cp README.md $(DIST_DIR)/windows/README.txt
	@echo "Creating Windows batch installer script..."
	@echo '@echo off' > $(DIST_DIR)/windows/install.bat
	@echo 'echo Installing HDR Oxide...' >> $(DIST_DIR)/windows/install.bat
	@echo 'set "INSTALL_DIR=%ProgramFiles%\HDR-Oxide"' >> $(DIST_DIR)/windows/install.bat
	@echo 'mkdir "%INSTALL_DIR%" 2>nul' >> $(DIST_DIR)/windows/install.bat
	@echo 'copy /Y HDR-Oxide.exe "%INSTALL_DIR%\"' >> $(DIST_DIR)/windows/install.bat
	@echo 'copy /Y LICENSE.txt "%INSTALL_DIR%\"' >> $(DIST_DIR)/windows/install.bat
	@echo 'copy /Y README.txt "%INSTALL_DIR%\"' >> $(DIST_DIR)/windows/install.bat
	@echo 'echo Installation complete!' >> $(DIST_DIR)/windows/install.bat
	@echo 'echo To run: "%INSTALL_DIR%\HDR-Oxide.exe"' >> $(DIST_DIR)/windows/install.bat
	@echo 'pause' >> $(DIST_DIR)/windows/install.bat
	@echo "Creating ZIP archive..."
	@which zip > /dev/null 2>&1 && cd $(DIST_DIR)/windows && zip -r ../HDR-Oxide-$(APP_VERSION)-windows.zip . || echo "Note: zip not available. Files ready in $(DIST_DIR)/windows/"
	@echo "Windows distribution created: $(DIST_DIR)/HDR-Oxide-$(APP_VERSION)-windows.zip"

dist: dist-mac dist-linux dist-windows
	@echo "All distributions created in $(DIST_DIR)/"

# Development helpers
fmt:
	cargo fmt

fmt-check:
	cargo fmt -- --check

doc:
	cargo doc --open
