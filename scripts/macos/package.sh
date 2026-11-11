#!/bin/bash

set -e

ARCH="${1:-aarch64-apple-darwin}" # or x86_64-apple-darwin
PACKAGE_NAME="otty"
VOLUME_NAME="OTTY"
RELEASE_DIR="target/release/macos"
APP_BUNDLE_DIR="$RELEASE_DIR/${VOLUME_NAME}.app"

PACKAGE_ID="$(cargo pkgid --package "$PACKAGE_NAME")"
# cargo pkgid ends in "#<version>" when the crate folder has the package
# name (crates/app/otty) and in "#<name>@<version>" otherwise.
APP_VERSION="${PACKAGE_ID##*[#@]}"

# The Homebrew tap downloads the DMG by this name, so it stays lowercase.
DMG_NAME="${PACKAGE_NAME}-${APP_VERSION}-${ARCH}.dmg"
DMG_PATH="${RELEASE_DIR}/${DMG_NAME}"

# The DMG window shows the app next to a link to /Applications, so the user
# installs it by dragging one onto the other.
STAGING_DIR="$(mktemp -d)"
trap 'rm -rf "$STAGING_DIR"' EXIT
cp -Rp "$APP_BUNDLE_DIR" "$STAGING_DIR/"
ln -s /Applications "$STAGING_DIR/Applications"

echo "Creating DMG: ${DMG_PATH}"

hdiutil create \
  -volname "${VOLUME_NAME}" \
  -srcfolder "${STAGING_DIR}" \
  -format UDZO \
  -ov \
  "${DMG_PATH}"

echo "DMG created at: ${DMG_PATH}"
