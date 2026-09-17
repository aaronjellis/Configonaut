#!/usr/bin/env bash
# sync-catalog-baseline.sh — copy marketplace-catalog/catalog.json into the
# embedded baseline the app ships with. CI fails if the two differ.
#
# Usage:
#   ./scripts/sync-catalog-baseline.sh            # copy repo → baseline
#   ./scripts/sync-catalog-baseline.sh --pull     # first refresh the repo copy
#                                                 # from the configonaut-catalog
#                                                 # GitHub repo, then copy
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$ROOT/marketplace-catalog/catalog.json"
DEST="$ROOT/tauri-app/src-tauri/resources/catalog-baseline.json"
REMOTE="https://raw.githubusercontent.com/aaronjellis/configonaut-catalog/main/catalog.json"

if [[ "${1:-}" == "--pull" ]]; then
  echo "Fetching $REMOTE"
  curl -fsSL "$REMOTE" -o "$SRC"
fi

jq empty "$SRC"   # fail fast on invalid JSON
cp "$SRC" "$DEST"
echo "Synced $(jq '.servers | length' "$SRC") servers → $DEST"
