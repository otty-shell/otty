#!/bin/bash

set -e

ARCH="${1:-aarch64-apple-darwin}" # or x86_64-apple-darwin
TARGET_BIN="otty"
ASSETS_DIR="assets"
RELEASE_DIR="target/release"
APP_NAME="otty.app"
APP_TEMPLATE="$ASSETS_DIR/packages/macos/$APP_NAME"
APP_DIR="$RELEASE_DIR/macos"
APP_BUNDLE="$APP_DIR/$APP_NAME"
APP_PLIST="$APP_BUNDLE/Contents/Info.plist"
APP_BINARY="$RELEASE_DIR/$TARGET_BIN"
APP_BINARY_DIR="$APP_BUNDLE/Contents/MacOS"
APP_EXTRAS_DIR="$APP_BUNDLE/Contents/Resources"

PACKAGE_ID="$(cargo pkgid --package "$TARGET_BIN")"
APP_VERSION="${PACKAGE_ID##*@}"
MARKETING_VERSION="${APP_VERSION%%[-+]*}"
BUILD_NUMBER="${OTTY_BUILD_NUMBER:-1}"

if [[ ! "$MARKETING_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Application version '$APP_VERSION' does not have a valid SemVer core" >&2
  exit 1
fi

if [[ ! "$BUILD_NUMBER" =~ ^[0-9]+$ ]]; then
  echo "OTTY_BUILD_NUMBER must contain only digits" >&2
  exit 1
fi

# build binary
lipo "target/$ARCH/release/$TARGET_BIN" -create -output "$APP_BINARY"

# build app
mkdir -p "$APP_DIR"
cp -fRp "$APP_TEMPLATE" "$APP_DIR"

/usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $MARKETING_VERSION" "$APP_PLIST"
/usr/libexec/PlistBuddy -c "Set :CFBundleVersion $BUILD_NUMBER" "$APP_PLIST"

mkdir -p "$APP_BINARY_DIR"
mkdir -p "$APP_EXTRAS_DIR"
cp -fp "$APP_BINARY" "$APP_BINARY_DIR"
touch -r "$APP_BINARY" "$APP_BUNDLE"
echo "Created '$APP_NAME' in '$APP_DIR'"
