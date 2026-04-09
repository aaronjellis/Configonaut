#!/bin/bash
set -e

APP_NAME="Configonaut"
DMG_NAME="${APP_NAME}-Installer"
VERSION="1.1.0"
VOLUME_NAME="${APP_NAME}"

cd "$(dirname "$0")"

# Step 1: Build the release app if not already built
if [ ! -d "${APP_NAME}.app" ]; then
    echo "Building ${APP_NAME}.app first..."
    bash build.sh
fi

echo "Packaging ${APP_NAME} v${VERSION} as DMG..."
rm -f "${DMG_NAME}.dmg" dmg-rw.dmg
STAGING="dmg-staging"
rm -rf "$STAGING"

# Step 2: Stage files with saved Finder layout
mkdir -p "$STAGING/.background"
cp -R "${APP_NAME}.app" "$STAGING/"
ln -s /Applications "$STAGING/Applications"
cp Resources/InstallerBG-dmg.png "$STAGING/.background/bg.png"
cp Resources/AppIcon.icns "$STAGING/.VolumeIcon.icns"
cp Resources/DS_Store.template "$STAGING/.DS_Store"

# Step 3: Create read-write DMG, set volume icon flag, convert to final
hdiutil create -volname "$VOLUME_NAME" -srcfolder "$STAGING" \
    -ov -format UDRW -fs HFS+ -size 40m dmg-rw.dmg -quiet
rm -rf "$STAGING"

hdiutil attach dmg-rw.dmg -readwrite -noverify -quiet
SetFile -a C "/Volumes/${VOLUME_NAME}" 2>/dev/null || true
sync
hdiutil detach "/Volumes/${VOLUME_NAME}" -quiet

hdiutil convert dmg-rw.dmg -format UDZO -imagekey zlib-level=9 \
    -o "${DMG_NAME}.dmg" -ov -quiet
rm -f dmg-rw.dmg

DMG_SIZE=$(du -h "${DMG_NAME}.dmg" | cut -f1 | xargs)
echo ""
echo "DMG created: $(pwd)/${DMG_NAME}.dmg  (${DMG_SIZE})"
echo ""
echo "Users can:"
echo "  1. Double-click ${DMG_NAME}.dmg"
echo "  2. Drag ${APP_NAME} to Applications"
echo "  3. Eject the disk image"
echo "  4. Launch from Applications or Spotlight"
