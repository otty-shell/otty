#!/bin/bash

set -e

ARCH="${1:-aarch64-apple-darwin}" # or x86_64-apple-darwin
APP_NAME="otty"
RELEASE_DIR="target/release/macos"
APP_BUNDLE_DIR="$RELEASE_DIR/${APP_NAME}.app"

PACKAGE_ID="$(cargo pkgid --package "$APP_NAME")"
APP_VERSION="${PACKAGE_ID##*@}"

DMG_NAME="${APP_NAME}-${APP_VERSION}-${ARCH}.dmg"
DMG_PATH="${RELEASE_DIR}/${DMG_NAME}"

echo "Creating DMG: ${DMG_PATH}"

hdiutil create \
  -volname "${APP_NAME}" \
  -srcfolder "${APP_BUNDLE_DIR}" \
  -format UDZO \
  -ov \
  "${DMG_PATH}"

echo "DMG created at: ${DMG_PATH}"
