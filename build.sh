#!/bin/bash
set -e

# Code signing identity and notarization profile
SIGN_IDENTITY="Developer ID Application: YOUR_NAME (YOUR_TEAM_ID)"
NOTARIZE_PROFILE="configonaut-notarize"

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
cp Resources/catalog-baseline.json "$APP/Contents/Resources/catalog-baseline.json"

# Copy bundled resources from SPM build (if SPM generated a resource bundle)
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
    <string>1.2.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.2.0</string>
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

# --- Code Signing ---
echo ""
echo "Signing $APP..."
codesign --deep --force --options runtime \
    --sign "$SIGN_IDENTITY" \
    "$APP"
echo "Verifying signature..."
codesign --verify --deep --strict "$APP"
echo "Signature valid."

# --- Notarization ---
# Skip notarization for fast local dev iterations:
#   SKIP_NOTARIZE=1 ./build.sh
if [ -n "$SKIP_NOTARIZE" ]; then
    echo ""
    echo "SKIP_NOTARIZE set — skipping notarization."
    echo ""
    echo "Build complete: $(pwd)/$APP  (signed, NOT notarized)"
    echo ""
    echo "To run now:  open '$APP'"
    exit 0
fi

echo ""
echo "Notarizing $APP (this may take a few minutes)..."
# Create a zip for notarization submission
ZIP_FOR_NOTARIZE="${APP%.app}-notarize.zip"
ditto -c -k --keepParent "$APP" "$ZIP_FOR_NOTARIZE"

xcrun notarytool submit "$ZIP_FOR_NOTARIZE" \
    --keychain-profile "$NOTARIZE_PROFILE" \
    --wait

rm -f "$ZIP_FOR_NOTARIZE"

# Staple the notarization ticket to the app
echo "Stapling notarization ticket..."
xcrun stapler staple "$APP"
echo "Notarization complete."

echo ""
echo "Build complete: $(pwd)/$APP  (signed + notarized)"
echo ""
echo "To install:  drag 'Configonaut.app' to /Applications"
echo "To run now:  open '$APP'"
