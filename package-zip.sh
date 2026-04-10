#!/bin/bash
set -e

APP_NAME="Configonaut"
VERSION="1.2.0"
ZIP_NAME="${APP_NAME}-${VERSION}.zip"
SIGN_IDENTITY="Developer ID Application: YOUR_NAME (YOUR_TEAM_ID)"
NOTARIZE_PROFILE="configonaut-notarize"

cd "$(dirname "$0")"

# Step 1: Build + sign + notarize + staple the app bundle (via build.sh)
# Always rebuild to pick up source changes (including MoveToApplications.swift).
echo "Building ${APP_NAME}.app..."
bash build.sh

echo ""
echo "Packaging ${APP_NAME} v${VERSION} as a zip..."
rm -f "${ZIP_NAME}"

# Step 2: Zip the already-notarized + stapled .app.
# Using ditto preserves extended attributes, code signature, and the
# stapled notarization ticket — recipients can double-click to extract
# and launch without Gatekeeper warnings.
ditto -c -k --keepParent "${APP_NAME}.app" "${ZIP_NAME}"

ZIP_SIZE=$(du -h "${ZIP_NAME}" | cut -f1 | xargs)
echo ""
echo "Zip created: $(pwd)/${ZIP_NAME}  (${ZIP_SIZE})"
echo ""
echo "Users can:"
echo "  1. Download ${ZIP_NAME}"
echo "  2. Double-click to extract ${APP_NAME}.app"
echo "  3. Double-click ${APP_NAME}.app to launch"
echo "  4. On first launch, Configonaut will offer to move itself to /Applications"
