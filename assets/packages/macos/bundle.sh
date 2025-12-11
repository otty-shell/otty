#!/usr/bin/env bash
set -euo pipefail

ARCH="${1:-aarch64-apple-darwin}" # или x86_64-apple-darwin
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")../../.." && pwd)"
TARGET_DIR="${PROJECT_ROOT}/target/${ARCH}/release"
APP_NAME="otty"
APP_BUNDLE_DIR="${PROJECT_ROOT}/dist/mac/${ARCH}/${APP_NAME}.app"

echo "Building app bundle for ${ARCH}"

mkdir -p "${APP_BUNDLE_DIR}/Contents/MacOS"
mkdir -p "${APP_BUNDLE_DIR}/Contents/Resources"

cp "${TARGET_DIR}/${APP_NAME}" "${APP_BUNDLE_DIR}/Contents/MacOS/${APP_NAME}"
cp "${PROJECT_ROOT}/packages/macos/Info.plist" "${APP_BUNDLE_DIR}/Contents/Info.plist"
cp "${PROJECT_ROOT}/packages/macos/logo.icns" "${APP_BUNDLE_DIR}/Contents/Resources/logo.icns"

echo "Created app bundle at: ${APP_BUNDLE_DIR}"
