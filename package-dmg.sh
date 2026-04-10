#!/bin/bash
set -e

APP_NAME="Configonaut"
DMG_NAME="${APP_NAME}-Installer"
VERSION="1.2.0"
VOLUME_NAME="${APP_NAME}"
SIGN_IDENTITY="Developer ID Application: YOUR_NAME (YOUR_TEAM_ID)"
NOTARIZE_PROFILE="configonaut-notarize"

cd "$(dirname "$0")"

# Step 1: Build the release app if not already built
if [ ! -d "${APP_NAME}.app" ]; then
    echo "Building ${APP_NAME}.app first..."
    bash build.sh
fi

echo "Packaging ${APP_NAME} v${VERSION} as DMG..."

# Clean up any stale mounts or previous artifacts
for vol in "/Volumes/${VOLUME_NAME}" "/Volumes/${VOLUME_NAME} 1" "/Volumes/${VOLUME_NAME} 2"; do
    if [ -d "$vol" ]; then
        hdiutil detach "$vol" -force -quiet 2>/dev/null || true
    fi
done
rm -f "${DMG_NAME}.dmg" "${DMG_NAME}-temp.dmg"

# Step 2: Create DMG with create-dmg (battle-tested homebrew tool)
# This handles background image, icon positions, and volume icon properly
# without the fragile Python DS_Store alias approach.
create-dmg \
    --volname "$VOLUME_NAME" \
    --volicon "Resources/AppIcon.icns" \
    --background "Resources/InstallerBG-dmg.png" \
    --window-pos 200 200 \
    --window-size 600 400 \
    --icon-size 128 \
    --icon "${APP_NAME}.app" 170 188 \
    --hide-extension "${APP_NAME}.app" \
    --app-drop-link 430 188 \
    --no-internet-enable \
    "${DMG_NAME}.dmg" \
    "${APP_NAME}.app"

# Strip any stale FinderInfo metadata
xattr -d com.apple.FinderInfo "${DMG_NAME}.dmg" 2>/dev/null || true

# Step 3: Sign the DMG
echo "Signing DMG..."
codesign --force --sign "$SIGN_IDENTITY" "${DMG_NAME}.dmg"

# Step 4: Notarize the DMG
echo "Notarizing DMG (this may take a few minutes)..."
xcrun notarytool submit "${DMG_NAME}.dmg" \
    --keychain-profile "$NOTARIZE_PROFILE" \
    --wait

echo "Stapling notarization ticket to DMG..."
xcrun stapler staple "${DMG_NAME}.dmg"

DMG_SIZE=$(du -h "${DMG_NAME}.dmg" | cut -f1 | xargs)
echo ""
echo "DMG created: $(pwd)/${DMG_NAME}.dmg  (${DMG_SIZE})  [signed + notarized]"
echo ""
echo "Users can:"
echo "  1. Double-click ${DMG_NAME}.dmg"
echo "  2. Drag ${APP_NAME} to Applications"
echo "  3. Eject the disk image"
echo "  4. Launch from Applications or Spotlight"
