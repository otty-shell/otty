#!/bin/bash

set -e

ARCH="${1:-aarch64-apple-darwin}" # or x86_64-apple-darwin
TARGET_BIN="otty"
ASSETS_DIR="assets"
RELEASE_DIR="target/release"
APP_NAME="otty.app"
APP_TEMPLATE="$ASSETS_DIR/packages/macos/$APP_NAME"
APP_TEMPLATE_PLIST="$APP_TEMPLATE/Contents/Info.plist"
APP_DIR="$RELEASE_DIR/macos"
APP_BINARY="$RELEASE_DIR/$TARGET_BIN"
APP_BINARY_DIR="$APP_DIR/$APP_NAME/Contents/MacOS"
APP_EXTRAS_DIR="$APP_DIR/$APP_NAME/Contents/Resources"

DMG_NAME="otty.dmg"
DMG_DIR="$RELEASE_DIR/macos"

VERSION=$(cat VERSION)
BUILD=$(git describe --always --dirty --exclude='*')

# update version and build
sed -i '' -e "s/{{ VERSION }}/$VERSION/g" "$APP_TEMPLATE_PLIST"
sed -i '' -e "s/{{ BUILD }}/$BUILD/g" "$APP_TEMPLATE_PLIST"

# build binary
lipo "target/$ARCH/release/$TARGET_BIN" -create -output "$APP_BINARY"

# build app
mkdir -p "$APP_BINARY_DIR"
mkdir -p "$APP_EXTRAS_DIR"
cp -fRp "$APP_TEMPLATE" "$APP_DIR"
cp -fp "$APP_BINARY" "$APP_BINARY_DIR"
touch -r "$APP_BINARY" "$APP_DIR/$APP_NAME"
echo "Created '$APP_NAME' in '$APP_DIR'"
