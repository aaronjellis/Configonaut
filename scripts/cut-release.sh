#!/usr/bin/env bash
# cut-release.sh — Bump version, update changelog, commit, tag, and push.
#
# Usage:
#   ./scripts/cut-release.sh 0.2.5
#
# What it does:
#   1. Validates the version argument
#   2. Bumps version in all 4 locations (tauri.conf.json, Cargo.toml, package.json, App.tsx)
#   3. Rebuilds Cargo.lock with the new version
#   4. Prompts you to verify CHANGELOG.md has the new section
#   5. Commits, tags, and pushes — triggering the release workflow

set -euo pipefail

VERSION="${1:-}"
if [[ -z "$VERSION" ]]; then
  echo "Usage: $0 <version>  (e.g. 0.2.5)"
  exit 1
fi

if [[ ! "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Error: version must be semver (e.g. 0.2.5), got: $VERSION"
  exit 1
fi

TAG="v$VERSION"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Check for clean working tree (allow untracked files).
if ! git -C "$ROOT" diff --quiet || ! git -C "$ROOT" diff --cached --quiet; then
  echo "Error: working tree has uncommitted changes. Commit or stash first."
  exit 1
fi

echo "Bumping to $VERSION..."

# 1. tauri.conf.json
sed -i '' "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" \
  "$ROOT/tauri-app/src-tauri/tauri.conf.json"

# 2. Cargo.toml — only the [package] version, not dependency versions.
# NOTE: `0,/pat/` is a GNU sed extension. BSD sed (macOS) silently accepts
# it but does nothing, which is why 0.2.5 and 0.2.6 ships left this file
# on 0.2.4. Scope the substitution to the [package] section via a range
# address — portable and still ignores the dozen other `version = ` lines
# in the dependencies block.
sed -i '' -E "/^\[package\]/,/^\[/ s/^version = \".*\"/version = \"$VERSION\"/" \
  "$ROOT/tauri-app/src-tauri/Cargo.toml"

# 3. package.json
sed -i '' "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" \
  "$ROOT/tauri-app/package.json"

# 4. App.tsx hardcoded version strings
sed -i '' "s/version=\"[^\"]*\"/version=\"$VERSION\"/g" \
  "$ROOT/tauri-app/src/App.tsx"

# 5. Rebuild Cargo.lock
(cd "$ROOT/tauri-app/src-tauri" && cargo check --quiet 2>/dev/null)

# 6. Check CHANGELOG.md has a section for this version
if ! grep -q "^## $VERSION" "$ROOT/CHANGELOG.md"; then
  echo ""
  echo "WARNING: CHANGELOG.md does not have a '## $VERSION' section."
  echo "Please add release notes before continuing."
  echo ""
  read -rp "Open CHANGELOG.md and continue? [y/N] " yn
  if [[ "$yn" != "y" && "$yn" != "Y" ]]; then
    echo "Aborted. Version files have been updated — commit manually when ready."
    exit 1
  fi
fi

# 7. Stage, commit, tag, push
git -C "$ROOT" add \
  tauri-app/src-tauri/tauri.conf.json \
  tauri-app/src-tauri/Cargo.toml \
  tauri-app/src-tauri/Cargo.lock \
  tauri-app/package.json \
  tauri-app/src/App.tsx \
  CHANGELOG.md

git -C "$ROOT" commit -m "chore: bump $TAG"
git -C "$ROOT" tag "$TAG"
git -C "$ROOT" push origin main "$TAG"

echo ""
echo "Done! Tag $TAG pushed. Release workflow should be running."
echo "Check: https://github.com/aaronjellis/Configonaut/actions"
