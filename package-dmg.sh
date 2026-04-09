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
rm -f "${DMG_NAME}.dmg" dmg-rw.dmg
STAGING="dmg-staging"
rm -rf "$STAGING"

# Step 2: Stage files
mkdir -p "$STAGING/.background"
cp -R "${APP_NAME}.app" "$STAGING/"
ln -s /Applications "$STAGING/Applications"
cp Resources/InstallerBG-dmg.png "$STAGING/.background/bg.png"
cp Resources/AppIcon.icns "$STAGING/.VolumeIcon.icns"

# Step 3: Create read-write DMG
hdiutil create -volname "$VOLUME_NAME" -srcfolder "$STAGING" \
    -ov -format UDRW -fs HFS+ -size 40m dmg-rw.dmg -quiet
rm -rf "$STAGING"

# Step 4: Mount, set icon flag, and write DS_Store with portable alias
hdiutil attach dmg-rw.dmg -readwrite -noverify -quiet
MOUNT="/Volumes/${VOLUME_NAME}"
for i in 1 2 3 4 5; do [ -d "$MOUNT" ] && break; sleep 1; done

SetFile -a C "$MOUNT" 2>/dev/null || true

# Write DS_Store with background alias generated on the mounted volume
# This ensures the alias is portable across machines
python3 << PYEOF
import plistlib
from ds_store import DSStore
from mac_alias import Alias

mount = "${MOUNT}"
d = DSStore.open(mount + "/.DS_Store", "w+")

# Background alias -- created from the mounted volume so it's portable
bg_alias = Alias.for_file(mount + "/.background/bg.png")

# Window settings (no toolbar, path bar, sidebar, etc.)
bwsp = {
    "ShowSidebar": False,
    "ShowToolbar": False,
    "ShowTabView": False,
    "ShowStatusBar": False,
    "ShowPathbar": False,
    "ContainerShowSidebar": False,
    "PreviewPaneVisibility": False,
    "WindowBounds": "{{200, 200}, {600, 400}}",
    "SidebarWidth": 0,
}
d["."]["bwsp"] = ("blob", plistlib.dumps(bwsp, fmt=plistlib.FMT_BINARY))

# Icon view with background image
icvp = {
    "backgroundType": 2,
    "backgroundImageAlias": bg_alias.to_bytes(),
    "gridOffsetX": 0.0,
    "gridOffsetY": 0.0,
    "gridSpacing": 100.0,
    "iconSize": 128.0,
    "labelOnBottom": True,
    "showIconPreview": True,
    "showItemInfo": False,
    "textSize": 14.0,
    "viewOptionsVersion": 1,
    "arrangeBy": "none",
}
d["."]["icvp"] = ("blob", plistlib.dumps(icvp, fmt=plistlib.FMT_BINARY))
d["."]["vSrn"] = ("long", 1)

# Icon positions matching the background circles
# 200px circles, 64px from edges, 72px gap
d["${APP_NAME}.app"]["Iloc"] = (int(164), int(200))
d["Applications"]["Iloc"] = (int(436), int(200))
d[".background"]["Iloc"] = (int(999), int(999))
d[".VolumeIcon.icns"]["Iloc"] = (int(999), int(999))

d.flush()
d.close()
print("DS_Store written with portable background alias")
PYEOF

sync
hdiutil detach "$MOUNT" -quiet

# Step 5: Convert to compressed read-only
hdiutil convert dmg-rw.dmg -format UDZO -imagekey zlib-level=9 \
    -o "${DMG_NAME}.dmg" -ov -quiet
rm -f dmg-rw.dmg

# Step 6: Sign the DMG
echo "Signing DMG..."
codesign --force --sign "$SIGN_IDENTITY" "${DMG_NAME}.dmg"

# Step 7: Notarize the DMG
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
