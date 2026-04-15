#!/usr/bin/env bash
# download-uv.sh — fetch uv sidecar binaries for all target platforms.
#
# These binaries are NOT committed to git (they are gitignored due to size).
# Run this script from the repo root or from tauri-app/src-tauri/binaries/
# before building or developing locally.
#
# Usage:
#   cd tauri-app/src-tauri/binaries
#   ./download-uv.sh

set -euo pipefail

UV_VER="0.11.6"
BASE_URL="https://github.com/astral-sh/uv/releases/download/${UV_VER}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Downloading uv ${UV_VER} sidecar binaries..."

download_tar() {
  local name="$1"   # e.g. uv-aarch64-apple-darwin
  local asset="$2"  # e.g. uv-aarch64-apple-darwin.tar.gz
  local binary="uv" # binary name inside the archive

  echo "  -> ${name}"
  TMP_DIR=$(mktemp -d)
  curl -fsSL "${BASE_URL}/${asset}" | tar -xz -C "${TMP_DIR}"
  cp "${TMP_DIR}/${name}/${binary}" "${SCRIPT_DIR}/${name}"
  chmod +x "${SCRIPT_DIR}/${name}"
  rm -rf "${TMP_DIR}"
}

download_zip() {
  local name="$1"   # e.g. uv-x86_64-pc-windows-msvc.exe
  local asset="$2"  # e.g. uv-x86_64-pc-windows-msvc.zip
  local binary="$3" # e.g. uv.exe

  echo "  -> ${name}"
  TMP_DIR=$(mktemp -d)
  curl -fsSL "${BASE_URL}/${asset}" -o "${TMP_DIR}/archive.zip"
  unzip -q "${TMP_DIR}/archive.zip" "${binary}" -d "${TMP_DIR}"
  cp "${TMP_DIR}/${binary}" "${SCRIPT_DIR}/${name}"
  chmod +x "${SCRIPT_DIR}/${name}"
  rm -rf "${TMP_DIR}"
}

download_tar "uv-aarch64-apple-darwin"      "uv-aarch64-apple-darwin.tar.gz"
download_tar "uv-x86_64-apple-darwin"       "uv-x86_64-apple-darwin.tar.gz"
download_zip "uv-x86_64-pc-windows-msvc.exe" "uv-x86_64-pc-windows-msvc.zip" "uv.exe"
download_tar "uv-x86_64-unknown-linux-gnu"  "uv-x86_64-unknown-linux-gnu.tar.gz"

echo "Done. Binaries written to ${SCRIPT_DIR}/"
ls -lh "${SCRIPT_DIR}"/uv-*
