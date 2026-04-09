#!/bin/bash
set -e

echo "Building Configonaut..."
cd "$(dirname "$0")"

swift build -c release 2>&1

APP="Configonaut.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources"

cp .build/release/Configonaut "$APP/Contents/MacOS/Configonaut"
cp Resources/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"
cp Resources/AppIcon.png "$APP/Contents/Resources/AppIcon.png"

# Copy bundled resources from SPM build
BUNDLE_PATH=$(find .build/release -name "Configonaut_Configonaut.bundle" -type d 2>/dev/null | head -1)
if [ -n "$BUNDLE_PATH" ]; then
    cp -R "$BUNDLE_PATH" "$APP/Contents/Resources/"
fi

cat > "$APP/Contents/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Configonaut</string>
    <key>CFBundleDisplayName</key>
    <string>Configonaut</string>
    <key>CFBundleIdentifier</key>
    <string>com.configonaut.app</string>
    <key>CFBundleVersion</key>
    <string>1.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.1.0</string>
    <key>CFBundleExecutable</key>
    <string>Configonaut</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticTermination</key>
    <true/>
    <key>NSSupportsSuddenTermination</key>
    <true/>
</dict>
</plist>
PLIST

echo ""
echo "Build complete: $(pwd)/$APP"
echo ""
echo "To install:  drag 'Configonaut.app' to /Applications"
echo "To run now:  open '$APP'"
