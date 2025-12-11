#!/usr/bin/env bash
set -euo pipefail

ARCH="${1:-aarch64-apple-darwin}" # или x86_64-apple-darwin
APP_NAME="otty"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
APP_BUNDLE_DIR="${PROJECT_ROOT}/dist/mac/${ARCH}/${APP_NAME}.app"
DMG_DIR="${PROJECT_ROOT}/dist/mac/${ARCH}"

mkdir -p "${DMG_DIR}"

DMG_NAME="${APP_NAME}-${ARCH}.dmg"
DMG_PATH="${DMG_DIR}/${DMG_NAME}"

echo "Creating DMG: ${DMG_PATH}"

hdiutil create \
  -volname "${APP_NAME}" \
  -srcfolder "${APP_BUNDLE_DIR}" \
  -format UDZO \
  -ov \
  "${DMG_PATH}"

echo "DMG created at: ${DMG_PATH}"

